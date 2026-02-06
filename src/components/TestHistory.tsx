import type { TestResult } from '../hooks/useTauri';

interface TestHistoryProps {
  results: TestResult[];
}

export function TestHistory({ results }: TestHistoryProps) {
  if (results.length === 0) return null;

  return (
    <section className="panel test-history">
      <h3>Recent Tests ({results.length})</h3>
      <table className="history-table">
        <thead>
          <tr>
            <th>Time</th>
            <th>Serial</th>
            <th>Verdict</th>
            <th>TTFF</th>
            <th>Sats</th>
          </tr>
        </thead>
        <tbody>
          {[...results].reverse().map((r, i) => (
            <tr key={i} className={r.verdict === 'pass' ? 'row-pass' : 'row-fail'}>
              <td>{new Date(r.timestamp).toLocaleTimeString()}</td>
              <td>{r.device_info.serial_number || '-'}</td>
              <td className={r.verdict === 'pass' ? 'cell-pass' : 'cell-fail'}>
                {r.verdict.toUpperCase()}
              </td>
              <td>{r.ttff_seconds?.toFixed(1) ?? '-'}s</td>
              <td>{r.best_gps_data?.satellites ?? '-'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}
