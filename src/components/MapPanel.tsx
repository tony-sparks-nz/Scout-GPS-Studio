import { useEffect, useRef, useState } from 'react';
import L from 'leaflet';
import 'leaflet/dist/leaflet.css';
import type { GpsData } from '../hooks/useTauri';

// Fix Leaflet default marker icon path (broken by bundlers)
import markerIcon2x from 'leaflet/dist/images/marker-icon-2x.png';
import markerIcon from 'leaflet/dist/images/marker-icon.png';
import markerShadow from 'leaflet/dist/images/marker-shadow.png';

delete (L.Icon.Default.prototype as any)._getIconUrl;
L.Icon.Default.mergeOptions({
  iconRetinaUrl: markerIcon2x,
  iconUrl: markerIcon,
  shadowUrl: markerShadow,
});

// All basemaps: CDN-backed, no API key, accessible from China
const BASEMAPS: Record<string, { url: string; attribution: string; subdomains?: string }> = {
  'Dark': {
    url: 'https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png',
    attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OSM</a> &copy; <a href="https://carto.com/">CARTO</a>',
    subdomains: 'abcd',
  },
  'Light': {
    url: 'https://{s}.basemaps.cartocdn.com/light_all/{z}/{x}/{y}{r}.png',
    attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OSM</a> &copy; <a href="https://carto.com/">CARTO</a>',
    subdomains: 'abcd',
  },
  'Voyager': {
    url: 'https://{s}.basemaps.cartocdn.com/rastertiles/voyager/{z}/{x}/{y}{r}.png',
    attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OSM</a> &copy; <a href="https://carto.com/">CARTO</a>',
    subdomains: 'abcd',
  },
  'Satellite': {
    url: 'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}',
    attribution: '&copy; Esri, Maxar, Earthstar Geographics',
  },
};

interface MapPanelProps {
  gpsData: GpsData | null;
}

export function MapPanel({ gpsData }: MapPanelProps) {
  const mapContainerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<L.Map | null>(null);
  const tileLayerRef = useRef<L.TileLayer | null>(null);
  const markerRef = useRef<L.Marker | null>(null);
  const accuracyCircleRef = useRef<L.Circle | null>(null);
  const hasCenteredRef = useRef(false);
  const [activeBasemap, setActiveBasemap] = useState('Satellite');

  // Initialize map once
  useEffect(() => {
    if (!mapContainerRef.current || mapRef.current) return;

    const map = L.map(mapContainerRef.current, {
      center: [30, 114], // Default: central China
      zoom: 3,
      zoomControl: true,
      attributionControl: true,
    });

    const basemap = BASEMAPS[activeBasemap];
    tileLayerRef.current = L.tileLayer(basemap.url, {
      attribution: basemap.attribution,
      subdomains: basemap.subdomains || 'abc',
      maxZoom: 19,
    }).addTo(map);

    mapRef.current = map;

    // Leaflet needs to know when its container resizes (flex layouts settle after init)
    const ro = new ResizeObserver(() => {
      map.invalidateSize();
    });
    ro.observe(mapContainerRef.current);

    return () => {
      ro.disconnect();
      map.remove();
      mapRef.current = null;
      tileLayerRef.current = null;
    };
  }, []);

  // Switch basemap
  const handleBasemapChange = (name: string) => {
    const map = mapRef.current;
    if (!map) return;

    if (tileLayerRef.current) {
      map.removeLayer(tileLayerRef.current);
    }

    const basemap = BASEMAPS[name];
    tileLayerRef.current = L.tileLayer(basemap.url, {
      attribution: basemap.attribution,
      subdomains: basemap.subdomains || 'abc',
      maxZoom: 19,
    }).addTo(map);

    setActiveBasemap(name);
  };

  // Update marker when GPS data changes
  useEffect(() => {
    const map = mapRef.current;
    if (!map) return;

    const lat = gpsData?.latitude;
    const lon = gpsData?.longitude;

    if (lat != null && lon != null && lat !== 0 && lon !== 0) {
      const pos: L.LatLngExpression = [lat, lon];

      if (markerRef.current) {
        markerRef.current.setLatLng(pos);
      } else {
        markerRef.current = L.marker(pos).addTo(map);
      }

      // Centre map on first fix
      if (!hasCenteredRef.current) {
        hasCenteredRef.current = true;
        map.invalidateSize();
        map.setView(pos, 14);
      }

      // Update accuracy circle based on HDOP (rough estimate: HDOP × 5m)
      const hdop = gpsData?.hdop;
      if (hdop != null && hdop > 0) {
        const radiusMeters = hdop * 5;
        if (accuracyCircleRef.current) {
          accuracyCircleRef.current.setLatLng(pos);
          accuracyCircleRef.current.setRadius(radiusMeters);
        } else {
          accuracyCircleRef.current = L.circle(pos, {
            radius: radiusMeters,
            color: '#00ccff',
            fillColor: '#00ccff',
            fillOpacity: 0.15,
            weight: 1,
          }).addTo(map);
        }
      }

      // Update marker popup
      markerRef.current.bindPopup(
        `<b>GPS Fix</b><br/>` +
        `Lat: ${lat.toFixed(6)}<br/>` +
        `Lon: ${lon.toFixed(6)}<br/>` +
        (gpsData?.altitude != null ? `Alt: ${gpsData.altitude.toFixed(1)}m<br/>` : '') +
        (gpsData?.satellites != null ? `Sats: ${gpsData.satellites}<br/>` : '') +
        (gpsData?.hdop != null ? `HDOP: ${gpsData.hdop.toFixed(1)}` : '')
      );
    }
  }, [gpsData?.latitude, gpsData?.longitude, gpsData?.hdop, gpsData?.altitude, gpsData?.satellites]);

  const hasFix = gpsData?.latitude != null && gpsData?.longitude != null &&
    gpsData.latitude !== 0 && gpsData.longitude !== 0;

  return (
    <section className="panel map-panel">
      <h2>
        GPS Fix Location
        {hasFix ? (
          <span className="fix-coords">
            {gpsData!.latitude!.toFixed(5)}, {gpsData!.longitude!.toFixed(5)}
          </span>
        ) : (
          <span className="no-fix-label">Waiting for fix...</span>
        )}
      </h2>
      <div className="basemap-switcher">
        {Object.keys(BASEMAPS).map((name) => (
          <button
            key={name}
            className={`btn btn-small ${activeBasemap === name ? 'btn-active' : ''}`}
            onClick={() => handleBasemapChange(name)}
          >
            {name}
          </button>
        ))}
      </div>
      <div ref={mapContainerRef} className="map-container" />
    </section>
  );
}
