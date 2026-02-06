import { useState, useEffect, useRef, useCallback } from 'react';
import { getNmeaBuffer, clearNmeaBuffer, isTauri } from '../hooks/useTauri';

interface NmeaTrafficProps {
  visible: boolean;
}

export function NmeaTraffic({ visible }: NmeaTrafficProps) {
  const [buffer, setBuffer] = useState<string[]>([]);
  const [autoScroll, setAutoScroll] = useState(true);
  const [paused, setPaused] = useState(false);
  const trafficRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!visible || !isTauri() || paused) return;

    const poll = async () => {
      try {
        const data = await getNmeaBuffer();
        setBuffer(data);
      } catch {
        // ignore
      }
    };

    poll();
    const interval = setInterval(poll, 500);
    return () => clearInterval(interval);
  }, [visible, paused]);

  useEffect(() => {
    if (autoScroll && trafficRef.current) {
      trafficRef.current.scrollTop = trafficRef.current.scrollHeight;
    }
  }, [buffer, autoScroll]);

  const handleClear = useCallback(async () => {
    try {
      await clearNmeaBuffer();
      setBuffer([]);
    } catch {
      // ignore
    }
  }, []);

  if (!visible) return null;

  return (
    <section className="panel nmea-traffic">
      <div className="nmea-header">
        <h3>NMEA Traffic</h3>
        <div className="nmea-controls">
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={autoScroll}
              onChange={(e) => setAutoScroll(e.target.checked)}
            />
            Auto-scroll
          </label>
          <button className="btn btn-small" onClick={() => setPaused(!paused)}>
            {paused ? 'Resume' : 'Pause'}
          </button>
          <button className="btn btn-small" onClick={handleClear}>
            Clear
          </button>
        </div>
      </div>
      <div ref={trafficRef} className="nmea-content">
        {buffer.length === 0 ? (
          <span className="no-data">Waiting for NMEA data...</span>
        ) : (
          buffer.map((sentence, i) => (
            <div key={i} className="nmea-sentence">{sentence}</div>
          ))
        )}
      </div>
    </section>
  );
}
