export function connectionLocality(raw: string): {
  locality: 'local' | 'cloud';
  allowPrivate: boolean;
} {
  try {
    const host = new URL(raw).hostname.toLowerCase().replace(/^\[|\]$/g, '');
    const privateHost =
      host === 'localhost' ||
      host === '::1' ||
      host.startsWith('127.') ||
      host.startsWith('10.') ||
      host.startsWith('192.168.') ||
      /^172\.(1[6-9]|2\d|3[0-1])\./.test(host) ||
      host.startsWith('fc') ||
      host.startsWith('fd');
    return privateHost
      ? { locality: 'local', allowPrivate: true }
      : { locality: 'cloud', allowPrivate: false };
  } catch {
    return { locality: 'cloud', allowPrivate: false };
  }
}
