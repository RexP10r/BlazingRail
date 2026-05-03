import http from 'k6/http';
import { check } from 'k6';
import { Counter, Trend } from 'k6/metrics';

const http202 = new Counter('http_202_total');
const http400 = new Counter('http_400_total');
const http503 = new Counter('http_503_total');
const latencyP95 = new Trend('latency_p95_ms');

export const options = {
  stages: [
    { duration: '5s', target: 256 },   
    { duration: '10s', target: 2048 }, // i cant set more on my machine
    { duration: '5s', target: 0 }, 
  ],
  http: {
    noConnectionReuse: false,
    maxConnsPerHost: 256,   
    timeout: '2s',
  },
  thresholds: {
    http_503_total: ['count<50'],
    latency_p95_ms: ['p(95)<32'],
    http_req_failed: ['rate<0.01'],
  },
};

export default function () {
  const payload = JSON.stringify({
    event_type: 'load_test',
    payload: { ts: Date.now(), rnd: Math.random() }
  });

  const res = http.post('http://127.0.0.1:3000/v1/events', payload, {
	  headers: { 'Content-Type': 'application/json' },
	  noConnectionReuse: false,
  });

  if (res.status === 202) http202.add(1);
  else if (res.status === 400) http400.add(1);
  else if (res.status === 503) http503.add(1);
  
  latencyP95.add(res.timings.duration);

  check(res, {
    'handled': (r) => r.status === 202 || r.status === 400 || r.status === 503,
  });
}
