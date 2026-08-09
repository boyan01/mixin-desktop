import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:latlng/latlng.dart';
import 'package:map/map.dart' as map;

import '../../constants/assets.dart';
import '../../models/message_list_entry.dart';
import '../app_protocol_handler.dart';
import '../interactive_decorated_box.dart';
import '../mixin_image.dart';
import 'unknown_special_message.dart';

class LocationMessageItem extends StatelessWidget {
  const LocationMessageItem({
    required this.message,
    super.key,
  });

  final MessageListEntry message;

  @override
  Widget build(BuildContext context) {
    final location = _LocationData.parse(message.content);
    if (location == null) {
      return UnknownSpecialMessage(category: message.category);
    }
    final uri = location.googleMapsUri;
    return SizedBox(
      width: 260,
      height: 180,
      child: InteractiveDecoratedBox(
        onTap: () => context.openUri(uri),
        child: Stack(
          children: [
            map.MapLayout(
              controller: map.MapController(
                location: LatLng.degree(location.latitude, location.longitude),
              ),
              builder: (context, transformer) => map.TileLayer(
                builder: (context, x, y, z) {
                  final url =
                      'https://www.google.com/maps/vt/pb=!1m4!1m3!1i$z!2i$x!3i$y!2m3!1e0!2sm!3i420120488!3m7!2sen!5e1105!12m4!1e68!2m2!1sset!2sRoadmap!4e0!5m1!1e0!23i4111425';
                  return MixinImage.network(url);
                },
              ),
            ),
            Center(
              child: Padding(
                padding: const EdgeInsets.only(bottom: 16.5),
                child: SvgPicture.asset(MixinAssets.locationMark),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _LocationData {
  const _LocationData(this.latitude, this.longitude, this.name, this.address);
  final double latitude;
  final double longitude;
  final String name;
  final String address;
  String get label => address.isNotEmpty ? address : name;
  Uri get googleMapsUri => Uri.parse(
    address.isEmpty
        ? 'https://www.google.com/maps/place/@$latitude,$longitude,17z?hl=zh-CN'
        : 'https://www.google.com/maps/search/${Uri.encodeComponent(address)}/@$latitude,$longitude,17z?hl=zh-CN',
  );

  static _LocationData? parse(String raw) {
    try {
      final json = jsonDecode(raw) as Map<String, dynamic>;
      return _LocationData(
        (json['latitude'] as num).toDouble(),
        (json['longitude'] as num).toDouble(),
        json['name']?.toString() ?? '',
        json['address']?.toString() ?? '',
      );
    } on Object {
      return null;
    }
  }
}
