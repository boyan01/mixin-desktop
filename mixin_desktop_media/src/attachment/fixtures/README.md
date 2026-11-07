# Generated video fixture

`rotated_red_blue.mp4` is a synthetic one-second H.264 video. Its encoded frame
is 160 × 90, red on the left and blue on the right. The MP4 display matrix applies
a 90-degree counterclockwise rotation. Displayed output is 90 × 160, blue on top
and red below.

Generated once with FFmpeg 9.0.2:

```sh
ffmpeg -f lavfi -i color=c=red:s=160x90:r=25 \
  -f lavfi -i color=c=blue:s=80x90:r=25 \
  -filter_complex '[0:v][1:v]overlay=80:0' -t 1 \
  -c:v libx264 -pix_fmt yuv420p -movflags +faststart unrotated.mp4
ffmpeg -display_rotation:v:0 90 -i unrotated.mp4 \
  -c copy -movflags +faststart rotated_red_blue.mp4
```

The tests use the checked-in bytes and macOS AVFoundation/ImageIO. They do not
run FFmpeg or ffprobe, access user files, or require an audio/video device.
