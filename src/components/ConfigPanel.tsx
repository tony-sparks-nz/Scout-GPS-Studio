import { useState, useEffect, useCallback } from 'react';
import {
  getTestCriteria,
  setTestCriteria,
  resetTestCriteria,
  type TestCriteria,
} from '../hooks/useTauri';

interface ConfigPanelProps {
  visible: boolean;
  onClose: () => void;
  onCriteriaChanged: (criteria: TestCriteria) => void;
}

export function ConfigPanel({ visible, onClose, onCriteriaChanged }: ConfigPanelProps) {
  const [criteria, setCriteria] = useState<TestCriteria | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!visible) return;
    getTestCriteria().then(setCriteria).catch(console.error);
  }, [visible]);

  const handleSave = useCallback(async () => {
    if (!criteria) return;
    setSaving(true);
    try {
      await setTestCriteria(criteria);
      onCriteriaChanged(criteria);
      onClose();
    } catch (e) {
      console.error('Failed to save criteria:', e);
    } finally {
      setSaving(false);
    }
  }, [criteria, onClose, onCriteriaChanged]);

  const handleReset = useCallback(async () => {
    try {
      const defaults = await resetTestCriteria();
      setCriteria(defaults);
      onCriteriaChanged(defaults);
    } catch (e) {
      console.error('Failed to reset criteria:', e);
    }
  }, [onCriteriaChanged]);

  const update = (field: keyof TestCriteria, value: number) => {
    if (criteria) {
      setCriteria({ ...criteria, [field]: value });
    }
  };

  if (!visible || !criteria) return null;

  return (
    <div className="config-overlay" onClick={onClose}>
      <div className="config-modal" onClick={(e) => e.stopPropagation()}>
        <h2>Test Criteria Configuration</h2>

        <div className="config-grid">
          <label>Min Satellites</label>
          <input
            type="number"
            value={criteria.min_satellites}
            onChange={(e) => update('min_satellites', Number(e.target.value))}
            min={1} max={30}
          />

          <label>Max HDOP</label>
          <input
            type="number"
            step="0.1"
            value={criteria.max_hdop}
            onChange={(e) => update('max_hdop', Number(e.target.value))}
            min={0.1} max={20}
          />

          <label>Max PDOP</label>
          <input
            type="number"
            step="0.1"
            value={criteria.max_pdop}
            onChange={(e) => update('max_pdop', Number(e.target.value))}
            min={0.1} max={20}
          />

          <label>Min Avg SNR (dB)</label>
          <input
            type="number"
            step="0.5"
            value={criteria.min_avg_snr}
            onChange={(e) => update('min_avg_snr', Number(e.target.value))}
            min={0} max={60}
          />

          <label>Min Strong Sats (SNR&gt;=30)</label>
          <input
            type="number"
            value={criteria.min_strong_satellites}
            onChange={(e) => update('min_strong_satellites', Number(e.target.value))}
            min={0} max={20}
          />

          <label>Max TTFF (seconds)</label>
          <input
            type="number"
            value={criteria.max_ttff_seconds}
            onChange={(e) => update('max_ttff_seconds', Number(e.target.value))}
            min={5} max={300}
          />

          <label>Min Constellations</label>
          <input
            type="number"
            value={criteria.min_constellations}
            onChange={(e) => update('min_constellations', Number(e.target.value))}
            min={1} max={6}
          />

          <label>Min Fix Quality</label>
          <input
            type="number"
            value={criteria.min_fix_quality}
            onChange={(e) => update('min_fix_quality', Number(e.target.value))}
            min={0} max={8}
          />

          <label>Stability Duration (seconds)</label>
          <input
            type="number"
            value={criteria.stability_duration_seconds}
            onChange={(e) => update('stability_duration_seconds', Number(e.target.value))}
            min={1} max={120}
          />
        </div>

        <div className="config-actions">
          <button className="btn btn-secondary" onClick={handleReset}>
            Reset Defaults
          </button>
          <button className="btn btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={handleSave} disabled={saving}>
            {saving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  );
}
