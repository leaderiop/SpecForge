import type { Node, Edge } from '@xyflow/react';

export const entityGraphNodes: Node[] = [
  { id: 'rate_limited_auth', type: 'entity', position: { x: 400, y: 220 }, data: { label: 'rate_limited_auth', kind: 'behavior' } },
  { id: 'auth_token_expiry', type: 'entity', position: { x: 80, y: 100 }, data: { label: 'auth_token_expiry', kind: 'invariant' } },
  { id: 'rate_limit_per_ip', type: 'entity', position: { x: 80, y: 340 }, data: { label: 'rate_limit_per_ip', kind: 'invariant' } },
  { id: 'UserStore', type: 'entity', position: { x: 250, y: 440 }, data: { label: 'UserStore', kind: 'port' } },
  { id: 'TokenService', type: 'entity', position: { x: 450, y: 440 }, data: { label: 'TokenService', kind: 'port' } },
  { id: 'RateLimiter', type: 'entity', position: { x: 650, y: 440 }, data: { label: 'RateLimiter', kind: 'port' } },
  { id: 'auth_succeeded', type: 'entity', position: { x: 680, y: 100 }, data: { label: 'auth_succeeded', kind: 'event' } },
  { id: 'auth_failed', type: 'entity', position: { x: 680, y: 340 }, data: { label: 'auth_failed', kind: 'event' } },
];

export const entityGraphEdges: Edge[] = [
  { id: 'e1', source: 'rate_limited_auth', target: 'auth_token_expiry', label: 'invariants', type: 'default' },
  { id: 'e2', source: 'rate_limited_auth', target: 'rate_limit_per_ip', label: 'invariants', type: 'default' },
  { id: 'e3', source: 'rate_limited_auth', target: 'UserStore', label: 'ports', type: 'default' },
  { id: 'e4', source: 'rate_limited_auth', target: 'TokenService', label: 'ports', type: 'default' },
  { id: 'e5', source: 'rate_limited_auth', target: 'RateLimiter', label: 'ports', type: 'default' },
  { id: 'e6', source: 'rate_limited_auth', target: 'auth_succeeded', label: 'produces', type: 'default' },
  { id: 'e7', source: 'rate_limited_auth', target: 'auth_failed', label: 'produces', type: 'default' },
];
