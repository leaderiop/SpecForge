export function add(store, title) {
  store.push({ title });
  return store;
}

export function list(store) {
  return store;
}
