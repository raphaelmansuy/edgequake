import { describe, expect, it } from 'vitest';
import {
  IMAGE_UPLOAD_NEEDS_IMAGES_BODY,
  IMAGE_UPLOAD_NEEDS_IMAGES_TITLE,
  imageUploadGuidance,
  isBlockedImageUpload,
} from '../image-upload-guidance';

const textOnly = { embeds_images: false };
const imageCapable = { embeds_images: true };

describe('SPEC-183 image upload guidance', () => {
  it('warns on a workspace that cannot store images, and names the fix', () => {
    const guidance = imageUploadGuidance(textOnly);
    expect(guidance.warns).toBe(true);
    expect(guidance.title).toBe(IMAGE_UPLOAD_NEEDS_IMAGES_TITLE);
    // A refusal without a destination is the bug; the fix must be in the sentence.
    expect(guidance.body).toBe(IMAGE_UPLOAD_NEEDS_IMAGES_BODY);
    expect(guidance.body).toMatch(/new workspace/i);
  });

  it('says nothing on a workspace that embeds images', () => {
    expect(imageUploadGuidance(imageCapable).warns).toBe(false);
  });

  it('says nothing when the server has not stated the capability', () => {
    // Fail open: "not stated" is not "refused". A stale payload must not block uploads.
    expect(imageUploadGuidance(undefined).warns).toBe(false);
    expect(imageUploadGuidance(null).warns).toBe(false);
    expect(isBlockedImageUpload('a.png', 'image/png', undefined)).toBe(false);
    expect(isBlockedImageUpload('a.png', 'image/png', null)).toBe(false);
  });

  it('recognises a blocked image by MIME or by extension', () => {
    expect(isBlockedImageUpload('a.png', 'image/png', textOnly)).toBe(true);
    expect(isBlockedImageUpload('a.JPG', 'application/octet-stream', textOnly)).toBe(true);
    expect(isBlockedImageUpload('a.webp', undefined, textOnly)).toBe(true);
    expect(isBlockedImageUpload('report.pdf', 'application/pdf', textOnly)).toBe(false);
    expect(isBlockedImageUpload('notes.md', 'text/markdown', textOnly)).toBe(false);
  });

  it('blocks nothing once the workspace embeds images', () => {
    expect(isBlockedImageUpload('a.png', 'image/png', imageCapable)).toBe(false);
  });
});
