import { base64UrlToBuffer, bufferToBase64Url } from './webauthn.helpers';

describe('webauthn helpers', () => {
  it('roundtrips binary data through base64url', () => {
    const bytes = new Uint8Array([0, 1, 250, 255, 32, 64]);
    const encoded = bufferToBase64Url(bytes.buffer);
    expect(encoded).not.toContain('+');
    expect(encoded).not.toContain('/');
    expect(encoded).not.toContain('=');
    expect(new Uint8Array(base64UrlToBuffer(encoded))).toEqual(bytes);
  });

  it('decodes standard base64url challenges', () => {
    const decoded = base64UrlToBuffer('AQID');
    expect(new Uint8Array(decoded)).toEqual(new Uint8Array([1, 2, 3]));
  });
});
