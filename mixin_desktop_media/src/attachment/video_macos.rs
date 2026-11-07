//! AVFoundation preparation. Non-Send native objects stay on one request-owned thread.
use std::{
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use block2::RcBlock;
use objc2::{
    msg_send,
    rc::{autoreleasepool, Retained},
    AnyThread, Message,
};
use objc2_av_foundation::{
    AVAssetImageGenerator, AVAssetImageGeneratorResult, AVAssetTrack,
    AVAsynchronousKeyValueLoading, AVKeyValueStatus, AVMediaTypeVideo, AVURLAsset,
    NSValueAVFoundationExtensions,
};
use objc2_core_foundation::{CFMutableData, CFString, CGSize};
use objc2_core_graphics::CGImage;
use objc2_core_media::{kCMTimeZero, CMTime};
use objc2_foundation::{NSArray, NSError, NSString, NSValue, NSURL};
use objc2_image_io::CGImageDestination;
use tokio::sync::oneshot;

use super::VideoInfo;
use crate::{MediaError, MediaResult};

const DEADLINE: Duration = Duration::from_secs(15);

enum Event {
    AssetLoaded,
    TrackLoaded,
    Poster(MediaResult<Vec<u8>>),
    Cancel,
}

struct Cancellation(Sender<Event>);
impl Drop for Cancellation {
    fn drop(&mut self) {
        let _ = self.0.send(Event::Cancel);
    }
}

struct Loading {
    asset: Retained<AVURLAsset>,
    generator: Option<Retained<AVAssetImageGenerator>>,
}
impl Drop for Loading {
    fn drop(&mut self) {
        // These calls, creation, getters and final release all occur on the
        // same native worker. Only plain channel messages cross threads.
        unsafe {
            if let Some(generator) = &self.generator {
                generator.cancelAllCGImageGeneration();
            }
            self.asset.cancelLoading();
        }
    }
}

fn invalid(message: &'static str) -> MediaError {
    MediaError::InvalidAttachment(message.into())
}

pub(super) async fn probe(path: &str) -> MediaResult<VideoInfo> {
    let (cancellation, result) = start(path.to_owned(), || {})?;
    let result = tokio::time::timeout(DEADLINE, result)
        .await
        .map_err(|_| invalid("Video preparation timed out"))?
        .map_err(|_| invalid("Video preparation stopped"))?;
    drop(cancellation);
    result
}

fn start(
    path: String,
    before_load: impl FnOnce() + Send + 'static,
) -> MediaResult<(Cancellation, oneshot::Receiver<MediaResult<VideoInfo>>)> {
    let (events, receiver) = mpsc::channel();
    let cancellation = Cancellation(events.clone());
    let (completed, result) = oneshot::channel();
    std::thread::Builder::new()
        .name("mixin-video-prepare".into())
        .spawn(move || {
            before_load();
            // Native resources are released before completion is made observable.
            let result = autoreleasepool(|_| load(&path, events, receiver));
            let _ = completed.send(result);
        })?;
    Ok((cancellation, result))
}

fn load(path: &str, events: Sender<Event>, receiver: Receiver<Event>) -> MediaResult<VideoInfo> {
    if matches!(receiver.try_recv(), Ok(Event::Cancel)) {
        return Err(invalid("Video preparation canceled"));
    }
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    // AVURLAsset initialization does not synchronously load asset properties.
    let asset = unsafe { AVURLAsset::URLAssetWithURL_options(&url, None) };
    let mut loading = Loading {
        asset,
        generator: None,
    };
    let keys = NSArray::from_retained_slice(&[
        NSString::from_str("duration"),
        NSString::from_str("tracks"),
    ]);
    let sender = events.clone();
    let loaded = RcBlock::new(move || {
        let _ = sender.send(Event::AssetLoaded);
    });
    // The sendable block only captures a plain Rust channel sender. AVFoundation
    // copies/retains it until loading completes or cancellation completes it.
    unsafe {
        loading
            .asset
            .loadValuesAsynchronouslyForKeys_completionHandler(&keys, Some(&loaded));
    }
    let mut track: Option<Retained<AVAssetTrack>> = None;
    let mut metadata = None;
    loop {
        match receiver
            .recv()
            .map_err(|_| invalid("Video preparation canceled"))?
        {
            Event::Cancel => return Err(invalid("Video preparation canceled")),
            Event::AssetLoaded if track.is_none() => {
                for key in ["duration", "tracks"] {
                    require_loaded(&*loading.asset, key)?;
                }
                let seconds = unsafe { loading.asset.duration().seconds() };
                if !seconds.is_finite() || seconds <= 0.0 || seconds * 1000.0 > i64::MAX as f64 {
                    return Err(invalid("Video duration is invalid"));
                }
                let media_type = unsafe { AVMediaTypeVideo }
                    .ok_or_else(|| invalid("Video media type is unavailable"))?;
                // This getter cannot block after the tracks key completed. Keep
                // macOS 11 support; the newer track-load method requires macOS 12.
                #[expect(
                    deprecated,
                    reason = "asynchronously loaded tracks; retain macOS 11 compatibility"
                )]
                let tracks = unsafe { loading.asset.tracksWithMediaType(media_type) };
                let video = tracks
                    .firstObject()
                    .ok_or_else(|| invalid("The file has no supported video track"))?;
                metadata = Some((0, 0, (seconds * 1000.0).round().max(1.0) as i64));
                let keys = NSArray::from_retained_slice(&[
                    NSString::from_str("naturalSize"),
                    NSString::from_str("preferredTransform"),
                ]);
                let sender = events.clone();
                let loaded = RcBlock::new(move || {
                    let _ = sender.send(Event::TrackLoaded);
                });
                unsafe {
                    video.loadValuesAsynchronouslyForKeys_completionHandler(&keys, Some(&loaded));
                }
                track = Some(video);
            }
            Event::TrackLoaded if loading.generator.is_none() => {
                let Some(track) = &track else { continue };
                for key in ["naturalSize", "preferredTransform"] {
                    require_loaded(&**track, key)?;
                }
                let size = unsafe { track.naturalSize() };
                let transform = unsafe { track.preferredTransform() };
                let width = (size.width * transform.a).abs() + (size.height * transform.c).abs();
                let height = (size.width * transform.b).abs() + (size.height * transform.d).abs();
                if !width.is_finite()
                    || !height.is_finite()
                    || width <= 0.0
                    || height <= 0.0
                    || width > f64::from(i32::MAX)
                    || height > f64::from(i32::MAX)
                {
                    return Err(invalid("Video dimensions are invalid"));
                }
                let (_, _, duration) =
                    metadata.ok_or_else(|| invalid("Video metadata is incomplete"))?;
                metadata = Some((
                    width.round().max(1.0) as i32,
                    height.round().max(1.0) as i32,
                    duration,
                ));
                let generator = unsafe {
                    AVAssetImageGenerator::initWithAsset(
                        AVAssetImageGenerator::alloc(),
                        &loading.asset,
                    )
                };
                unsafe {
                    generator.setAppliesPreferredTrackTransform(true);
                    generator.setMaximumSize(CGSize::new(320.0, 320.0));
                }
                let sender = events.clone();
                let generated = RcBlock::new(
                    move |_: CMTime,
                          image: *mut CGImage,
                          _: CMTime,
                          status: AVAssetImageGeneratorResult,
                          _: *mut NSError| {
                        let result = if status == AVAssetImageGeneratorResult::Succeeded {
                            // The image is valid only during this native callback.
                            unsafe { image.as_ref() }
                                .ok_or_else(|| invalid("The video preview is empty"))
                                .and_then(encode_poster)
                        } else {
                            Err(invalid("Could not generate the video preview"))
                        };
                        let _ = sender.send(Event::Poster(result));
                    },
                );
                let time = unsafe { NSValue::valueWithCMTime(kCMTimeZero) };
                let times = NSArray::from_retained_slice(&[time]);
                unsafe {
                    generator.generateCGImagesAsynchronouslyForTimes_completionHandler(
                        &times,
                        RcBlock::as_ptr(&generated),
                    );
                }
                loading.generator = Some(generator);
            }
            Event::Poster(result) => {
                let (width, height, duration_millis) =
                    metadata.ok_or_else(|| invalid("Video metadata is incomplete"))?;
                return Ok(VideoInfo {
                    width,
                    height,
                    duration_millis,
                    thumbnail: Some(STANDARD.encode(result?)),
                });
            }
            _ => {}
        }
    }
}

fn require_loaded(object: &impl Message, key: &str) -> MediaResult<()> {
    let key = NSString::from_str(key);
    // This deprecated-in-Swift getter is still the documented Objective-C
    // completion contract and is missing from objc2's generated protocol.
    let status: AVKeyValueStatus = unsafe {
        msg_send![object, statusOfValueForKey: &*key, error: std::ptr::null_mut::<*mut NSError>()]
    };
    if status == AVKeyValueStatus::Loaded {
        Ok(())
    } else {
        Err(invalid("Could not read the video metadata"))
    }
}

fn encode_poster(image: &CGImage) -> MediaResult<Vec<u8>> {
    let data = CFMutableData::new(None, 0)
        .ok_or_else(|| invalid("Could not allocate the video preview"))?;
    // No generic properties are supplied. ImageIO writes a single JPEG frame.
    let destination = unsafe {
        CGImageDestination::with_data(&data, &CFString::from_str("public.jpeg"), 1, None)
    }
    .ok_or_else(|| invalid("Could not encode the video preview"))?;
    unsafe {
        destination.add_image(image, None);
        if !destination.finalize() {
            return Err(invalid("Could not encode the video preview"));
        }
    }
    Ok(data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disposing_a_request_before_its_worker_starts_cancels_native_preparation_and_finishes_the_worker(
    ) {
        use std::sync::{Arc, Barrier};
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("valid.mp4");
        std::fs::write(&source, include_bytes!("fixtures/rotated_red_blue.mp4")).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let worker_barrier = barrier.clone();
        let (cancellation, result) = start(source.to_string_lossy().into_owned(), move || {
            worker_barrier.wait();
        })
        .unwrap();
        drop(cancellation);
        barrier.wait();
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
            let result = tokio::time::timeout(Duration::from_secs(2), result).await.unwrap().unwrap();
            assert!(matches!(result, Err(MediaError::InvalidAttachment(message)) if message == "Video preparation canceled"));
        });
    }
}
