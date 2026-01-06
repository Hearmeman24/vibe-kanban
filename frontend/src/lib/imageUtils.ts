/**
 * Image utilities for avatar upload with face detection
 */

import * as faceapi from 'face-api.js';

// Track model loading state
let modelsLoaded = false;
let modelsLoading: Promise<void> | null = null;

/**
 * Face detection result from faceapi.js
 */
export interface FaceDetectionResult {
  x: number;
  y: number;
  width: number;
  height: number;
  score: number;
}

/**
 * Crop region definition
 */
export interface CropRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Result of image compression
 */
export interface CompressionResult {
  dataUrl: string;
  sizeBytes: number;
  width: number;
  height: number;
}

/**
 * Load face-api.js models from CDN
 * Models are loaded once and cached
 */
export async function loadFaceDetectionModels(): Promise<void> {
  if (modelsLoaded) {
    return;
  }

  if (modelsLoading) {
    return modelsLoading;
  }

  modelsLoading = (async () => {
    // Use jsdelivr CDN for face-api.js models
    const MODEL_URL =
      'https://cdn.jsdelivr.net/npm/@vladmandic/face-api@1.7.12/model';

    try {
      // Load the TinyFaceDetector model - lightweight and fast
      await faceapi.nets.tinyFaceDetector.loadFromUri(MODEL_URL);
      modelsLoaded = true;
    } catch (error) {
      modelsLoading = null;
      throw new Error(
        `Failed to load face detection models: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    }
  })();

  return modelsLoading;
}

/**
 * Check if face detection models are loaded
 */
export function areModelsLoaded(): boolean {
  return modelsLoaded;
}

/**
 * Detect faces in an image
 * @param image - HTMLImageElement or HTMLCanvasElement to detect faces in
 * @returns Array of detected faces sorted by score (highest first)
 */
export async function detectFaces(
  image: HTMLImageElement | HTMLCanvasElement
): Promise<FaceDetectionResult[]> {
  if (!modelsLoaded) {
    await loadFaceDetectionModels();
  }

  const detections = await faceapi.detectAllFaces(
    image,
    new faceapi.TinyFaceDetectorOptions({
      inputSize: 416,
      scoreThreshold: 0.5,
    })
  );

  return detections
    .map((detection) => ({
      x: detection.box.x,
      y: detection.box.y,
      width: detection.box.width,
      height: detection.box.height,
      score: detection.score,
    }))
    .sort((a, b) => b.score - a.score);
}

/**
 * Get the largest face from detection results
 * @param faces - Array of detected faces
 * @returns The largest face or null if no faces detected
 */
export function getLargestFace(
  faces: FaceDetectionResult[]
): FaceDetectionResult | null {
  if (faces.length === 0) {
    return null;
  }

  return faces.reduce((largest, current) => {
    const currentArea = current.width * current.height;
    const largestArea = largest.width * largest.height;
    return currentArea > largestArea ? current : largest;
  }, faces[0]);
}

/**
 * Calculate a square crop region centered on a face with padding
 * @param face - The detected face to center on
 * @param imageWidth - Width of the source image
 * @param imageHeight - Height of the source image
 * @param paddingPercent - Padding around the face as a percentage (0-100)
 * @returns Crop region that fits within image bounds
 */
export function calculateFaceCropRegion(
  face: FaceDetectionResult,
  imageWidth: number,
  imageHeight: number,
  paddingPercent: number = 50
): CropRegion {
  // Calculate face center
  const faceCenterX = face.x + face.width / 2;
  const faceCenterY = face.y + face.height / 2;

  // Use the larger dimension for a square crop base
  const faceSize = Math.max(face.width, face.height);

  // Add padding
  const paddingMultiplier = 1 + paddingPercent / 100;
  const cropSize = faceSize * paddingMultiplier;

  // Calculate initial crop region (centered on face)
  let cropX = faceCenterX - cropSize / 2;
  let cropY = faceCenterY - cropSize / 2;

  // Ensure crop region stays within image bounds
  cropX = Math.max(0, Math.min(cropX, imageWidth - cropSize));
  cropY = Math.max(0, Math.min(cropY, imageHeight - cropSize));

  // Adjust size if it exceeds image dimensions
  const finalSize = Math.min(
    cropSize,
    imageWidth - cropX,
    imageHeight - cropY,
    imageWidth,
    imageHeight
  );

  // Re-center if size was adjusted
  if (finalSize < cropSize) {
    cropX = Math.max(0, faceCenterX - finalSize / 2);
    cropY = Math.max(0, faceCenterY - finalSize / 2);

    // Final bounds check
    cropX = Math.min(cropX, imageWidth - finalSize);
    cropY = Math.min(cropY, imageHeight - finalSize);
  }

  return {
    x: Math.round(cropX),
    y: Math.round(cropY),
    width: Math.round(finalSize),
    height: Math.round(finalSize),
  };
}

/**
 * Calculate a center crop region for images without faces
 * @param imageWidth - Width of the source image
 * @param imageHeight - Height of the source image
 * @returns Square crop region centered on the image
 */
export function calculateCenterCropRegion(
  imageWidth: number,
  imageHeight: number
): CropRegion {
  const size = Math.min(imageWidth, imageHeight);
  return {
    x: Math.round((imageWidth - size) / 2),
    y: Math.round((imageHeight - size) / 2),
    width: size,
    height: size,
  };
}

/**
 * Crop an image to a specified region
 * @param image - Source image element
 * @param region - Region to crop
 * @returns Canvas with cropped image
 */
export function cropImage(
  image: HTMLImageElement,
  region: CropRegion
): HTMLCanvasElement {
  const canvas = document.createElement('canvas');
  canvas.width = region.width;
  canvas.height = region.height;

  const ctx = canvas.getContext('2d');
  if (!ctx) {
    throw new Error('Failed to get canvas context');
  }

  ctx.drawImage(
    image,
    region.x,
    region.y,
    region.width,
    region.height,
    0,
    0,
    region.width,
    region.height
  );

  return canvas;
}

/**
 * Compress an image to meet target file size
 * @param canvas - Canvas with the image to compress
 * @param targetSizeKB - Target file size in KB (default: 100KB)
 * @param minQuality - Minimum JPEG quality (default: 0.5)
 * @param maxQuality - Maximum JPEG quality (default: 0.92)
 * @param maxDimension - Maximum width/height (default: 512)
 * @returns Compressed image as data URL with metadata
 */
export function compressImage(
  canvas: HTMLCanvasElement,
  targetSizeKB: number = 100,
  minQuality: number = 0.5,
  maxQuality: number = 0.92,
  maxDimension: number = 512
): CompressionResult {
  const targetSizeBytes = targetSizeKB * 1024;

  // First, resize if needed
  let workingCanvas = canvas;
  let width = canvas.width;
  let height = canvas.height;

  if (width > maxDimension || height > maxDimension) {
    const scale = maxDimension / Math.max(width, height);
    width = Math.round(width * scale);
    height = Math.round(height * scale);

    workingCanvas = document.createElement('canvas');
    workingCanvas.width = width;
    workingCanvas.height = height;

    const ctx = workingCanvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get canvas context');
    }

    // Use better quality scaling
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';
    ctx.drawImage(canvas, 0, 0, width, height);
  }

  // Binary search for optimal quality
  let quality = maxQuality;
  let dataUrl = workingCanvas.toDataURL('image/jpeg', quality);
  let sizeBytes = getDataUrlSize(dataUrl);

  // If already under target, return as-is
  if (sizeBytes <= targetSizeBytes) {
    return { dataUrl, sizeBytes, width, height };
  }

  // Binary search for quality
  let low = minQuality;
  let high = maxQuality;

  for (let i = 0; i < 10; i++) {
    quality = (low + high) / 2;
    dataUrl = workingCanvas.toDataURL('image/jpeg', quality);
    sizeBytes = getDataUrlSize(dataUrl);

    if (sizeBytes <= targetSizeBytes) {
      low = quality;
    } else {
      high = quality;
    }

    // Close enough
    if (Math.abs(sizeBytes - targetSizeBytes) < targetSizeBytes * 0.1) {
      break;
    }
  }

  // Final pass with the best quality that fits
  dataUrl = workingCanvas.toDataURL('image/jpeg', low);
  sizeBytes = getDataUrlSize(dataUrl);

  return { dataUrl, sizeBytes, width, height };
}

/**
 * Calculate the approximate size in bytes of a data URL
 */
function getDataUrlSize(dataUrl: string): number {
  // Data URL format: data:image/jpeg;base64,<base64-data>
  const base64Index = dataUrl.indexOf(',') + 1;
  const base64Data = dataUrl.substring(base64Index);
  // Base64 encoding increases size by ~33%
  return Math.round((base64Data.length * 3) / 4);
}

/**
 * Load an image from a File object
 * @param file - File to load
 * @returns Promise resolving to an HTMLImageElement
 */
export function loadImageFromFile(file: File): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();

    reader.onload = (e) => {
      const img = new Image();

      img.onload = () => resolve(img);
      img.onerror = () => reject(new Error('Failed to load image'));

      img.src = e.target?.result as string;
    };

    reader.onerror = () => reject(new Error('Failed to read file'));
    reader.readAsDataURL(file);
  });
}

/**
 * Validate that a file is a supported image type
 * @param file - File to validate
 * @returns true if valid image type
 */
export function isValidImageFile(file: File): boolean {
  const validTypes = [
    'image/jpeg',
    'image/png',
    'image/gif',
    'image/webp',
    'image/bmp',
  ];
  return validTypes.includes(file.type);
}

/**
 * Maximum file size in bytes (10MB)
 */
export const MAX_FILE_SIZE = 10 * 1024 * 1024;

/**
 * Check if a file is within size limits
 * @param file - File to check
 * @returns true if within limits
 */
export function isFileSizeValid(file: File): boolean {
  return file.size <= MAX_FILE_SIZE;
}

/**
 * Draw face detection boxes on a canvas for preview
 * @param canvas - Canvas to draw on
 * @param faces - Detected faces
 * @param cropRegion - Optional current crop region to highlight
 */
export function drawFaceOverlay(
  canvas: HTMLCanvasElement,
  faces: FaceDetectionResult[],
  cropRegion?: CropRegion
): void {
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  // Draw face boxes
  ctx.strokeStyle = '#22c55e';
  ctx.lineWidth = 2;

  faces.forEach((face) => {
    ctx.strokeRect(face.x, face.y, face.width, face.height);
  });

  // Draw crop region if provided
  if (cropRegion) {
    // Dim areas outside crop region
    ctx.fillStyle = 'rgba(0, 0, 0, 0.5)';

    // Top
    ctx.fillRect(0, 0, canvas.width, cropRegion.y);
    // Bottom
    ctx.fillRect(
      0,
      cropRegion.y + cropRegion.height,
      canvas.width,
      canvas.height - cropRegion.y - cropRegion.height
    );
    // Left
    ctx.fillRect(0, cropRegion.y, cropRegion.x, cropRegion.height);
    // Right
    ctx.fillRect(
      cropRegion.x + cropRegion.width,
      cropRegion.y,
      canvas.width - cropRegion.x - cropRegion.width,
      cropRegion.height
    );

    // Crop border
    ctx.strokeStyle = '#3b82f6';
    ctx.lineWidth = 2;
    ctx.setLineDash([5, 5]);
    ctx.strokeRect(
      cropRegion.x,
      cropRegion.y,
      cropRegion.width,
      cropRegion.height
    );
    ctx.setLineDash([]);
  }
}
