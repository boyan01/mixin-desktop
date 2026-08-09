import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:mixin_desktop_ui/utils/cache_client.dart';

void main() {
  late Directory rootDirectory;

  setUp(() async {
    rootDirectory = await Directory.systemTemp.createTemp(
      'mixin-image-cache-test-',
    );
  });

  tearDown(() async {
    if (await rootDirectory.exists()) {
      await rootDirectory.delete(recursive: true);
    }
  });

  test('reuses cached bytes across cache client instances', () async {
    final url = Uri.parse('https://images.mixin.one/avatar.png');
    var fetchCount = 0;
    final firstClient = CacheClient(
      'cache_image',
      rootDirectory: rootDirectory,
    );
    final first = await firstClient.getOrFetchBytes(url, () async {
      fetchCount++;
      return Uint8List.fromList([1, 2, 3]);
    });
    firstClient.close();

    final secondClient = CacheClient(
      'cache_image',
      rootDirectory: rootDirectory,
    );
    final second = await secondClient.getOrFetchBytes(url, () async {
      fetchCount++;
      return Uint8List.fromList([4, 5, 6]);
    });
    secondClient.close();

    expect(first.fromCache, isFalse);
    expect(second.fromCache, isTrue);
    expect(second.bytes, [1, 2, 3]);
    expect(fetchCount, 1);
  });

  test('coalesces concurrent fetches for the same URL', () async {
    final client = CacheClient(
      'cache_image',
      rootDirectory: rootDirectory,
    );
    final fetchStarted = Completer<void>();
    final finishFetch = Completer<void>();
    var fetchCount = 0;

    Future<Uint8List> fetch() async {
      fetchCount++;
      fetchStarted.complete();
      await finishFetch.future;
      return Uint8List.fromList([7, 8, 9]);
    }

    final first = client.getOrFetchBytes(
      Uri.parse('https://images.mixin.one/sticker.webp'),
      fetch,
    );
    await fetchStarted.future;
    final second = client.getOrFetchBytes(
      Uri.parse('https://images.mixin.one/sticker.webp'),
      fetch,
    );
    finishFetch.complete();

    expect((await first).bytes, [7, 8, 9]);
    expect((await second).bytes, [7, 8, 9]);
    expect(fetchCount, 1);
    client.close();
  });

  test('evict forces the next access to fetch again', () async {
    final client = CacheClient(
      'cache_image',
      rootDirectory: rootDirectory,
    );
    final url = Uri.parse('https://images.mixin.one/broken.png');
    var fetchCount = 0;

    Future<Uint8List> fetch() async {
      fetchCount++;
      return Uint8List.fromList([fetchCount]);
    }

    expect((await client.getOrFetchBytes(url, fetch)).bytes, [1]);
    await client.evict(url);
    expect((await client.getOrFetchBytes(url, fetch)).bytes, [2]);
    expect(fetchCount, 2);
    client.close();
  });
}
