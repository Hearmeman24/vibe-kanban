import { useState, useRef, useEffect, useCallback } from 'react';
import * as faceapi from 'face-api.js';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Slider } from '@/components/ui/slider';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import {
  Loader2,
  Upload,
  AlertTriangle,
  User,
  RefreshCw,
  ImageIcon,
} from 'lucide-react';
import { defineModal } from '@/lib/modals';

export interface AvatarUploadDialogProps {
  agentName: string;
  currentAvatar?: string;
}

export type AvatarUploadResult =
  | { type: 'saved'; imageDataUrl: string }
  | { type: 'canceled' };

// Max file size: 5MB
const MAX_FILE_SIZE = 5 * 1024 * 1024;
const ACCEPTED_TYPES = ['image/png', 'image/jpeg', 'image/jpg'];

// Model loading state - shared across instances
let modelsLoaded = false;
let modelsLoading = false;
let modelLoadPromise: Promise<void> | null = null;

async function loadModels(): Promise<void> {
  if (modelsLoaded) return;
  if (modelsLoading && modelLoadPromise) return modelLoadPromise;

  modelsLoading = true;
  modelLoadPromise = (async () => {
    // Load models from CDN
    const MODEL_URL =
      'https://cdn.jsdelivr.net/npm/@vladmandic/face-api@1.7.12/model';

    await Promise.all([
      faceapi.nets.tinyFaceDetector.loadFromUri(MODEL_URL),
      faceapi.nets.faceLandmark68Net.loadFromUri(MODEL_URL),
    ]);

    modelsLoaded = true;
    modelsLoading = false;
  })();

  return modelLoadPromise;
}

interface FaceDetection {
  box: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
  score: number;
}

function compressImage(
  canvas: HTMLCanvasElement,
  maxSize: number = 100,
  quality: number = 0.85
): string {
  // Create output canvas at target size
  const outputCanvas = document.createElement('canvas');
  outputCanvas.width = maxSize;
  outputCanvas.height = maxSize;
  const ctx = outputCanvas.getContext('2d');

  if (!ctx) {
    throw new Error('Failed to get canvas context');
  }

  // Draw scaled image
  ctx.drawImage(canvas, 0, 0, maxSize, maxSize);

  // Return as data URL
  return outputCanvas.toDataURL('image/jpeg', quality);
}

// Helper function to draw crop preview
function drawCropPreview(
  img: HTMLImageElement,
  previewCanvas: HTMLCanvasElement,
  face: FaceDetection,
  padding: number
): void {
  const ctx = previewCanvas.getContext('2d');
  if (!ctx) return;

  // Calculate crop area with padding
  const cropSize = Math.max(face.box.width, face.box.height) + padding * 2;
  const cropX = face.box.x + face.box.width / 2 - cropSize / 2;
  const cropY = face.box.y + face.box.height / 2 - cropSize / 2;

  // Clamp to image bounds
  const clampedX = Math.max(0, Math.min(cropX, img.width - cropSize));
  const clampedY = Math.max(0, Math.min(cropY, img.height - cropSize));
  const clampedSize = Math.min(
    cropSize,
    img.width - clampedX,
    img.height - clampedY
  );

  // Set preview canvas size
  previewCanvas.width = 150;
  previewCanvas.height = 150;

  // Clear and draw cropped area
  ctx.clearRect(0, 0, 150, 150);

  // Save context state before clipping
  ctx.save();

  // Draw circular clip
  ctx.beginPath();
  ctx.arc(75, 75, 75, 0, Math.PI * 2);
  ctx.closePath();
  ctx.clip();

  ctx.drawImage(
    img,
    clampedX,
    clampedY,
    clampedSize,
    clampedSize,
    0,
    0,
    150,
    150
  );

  // Restore context state
  ctx.restore();
}

const AvatarUploadDialogImpl = NiceModal.create<AvatarUploadDialogProps>(
  (props) => {
    const modal = useModal();
    const { agentName, currentAvatar } = props;

    // State
    const [isLoadingModels, setIsLoadingModels] = useState(true);
    const [modelError, setModelError] = useState<string | null>(null);
    const [imageDataUrl, setImageDataUrl] = useState<string | null>(null);
    const [isDetecting, setIsDetecting] = useState(false);
    const [detectedFaces, setDetectedFaces] = useState<FaceDetection[]>([]);
    const [selectedFaceIndex, setSelectedFaceIndex] = useState<number>(0);
    const [padding, setPadding] = useState(30);
    const [error, setError] = useState<string | null>(null);

    // Refs
    const fileInputRef = useRef<HTMLInputElement>(null);
    const originalCanvasRef = useRef<HTMLCanvasElement>(null);
    const previewCanvasRef = useRef<HTMLCanvasElement>(null);
    const imageRef = useRef<HTMLImageElement | null>(null);

    // Load face-api models on mount
    useEffect(() => {
      let mounted = true;

      loadModels()
        .then(() => {
          if (mounted) {
            setIsLoadingModels(false);
          }
        })
        .catch((loadErr) => {
          if (mounted) {
            const errMsg =
              loadErr instanceof Error ? loadErr.message : 'Unknown error';
            setModelError('Failed to load face detection models: ' + errMsg);
            setIsLoadingModels(false);
          }
        });

      return () => {
        mounted = false;
      };
    }, []);

    // Handle file selection
    const handleFileSelect = useCallback(
      (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0];
        setError(null);
        setDetectedFaces([]);
        setSelectedFaceIndex(0);

        if (!file) return;

        // Validate file type
        if (!ACCEPTED_TYPES.includes(file.type)) {
          setError('Please select a PNG or JPEG image file.');
          return;
        }

        // Validate file size
        if (file.size > MAX_FILE_SIZE) {
          setError('File is too large. Maximum size is 5MB.');
          return;
        }

        // Read file as data URL
        const reader = new FileReader();
        reader.onload = (e) => {
          const dataUrl = e.target?.result as string;
          setImageDataUrl(dataUrl);
        };
        reader.onerror = () => {
          setError('Failed to read file. Please try again.');
        };
        reader.readAsDataURL(file);
      },
      []
    );

    // Detect faces when image loads
    useEffect(() => {
      if (!imageDataUrl || isLoadingModels || modelError) return;

      const img = new Image();
      img.onload = async () => {
        imageRef.current = img;

        // Draw original image on canvas
        const canvas = originalCanvasRef.current;
        if (!canvas) return;

        // Scale canvas to fit while maintaining aspect ratio
        const maxWidth = 400;
        const maxHeight = 300;
        let width = img.width;
        let height = img.height;

        if (width > maxWidth) {
          height = (height * maxWidth) / width;
          width = maxWidth;
        }
        if (height > maxHeight) {
          width = (width * maxHeight) / height;
          height = maxHeight;
        }

        canvas.width = width;
        canvas.height = height;

        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        ctx.drawImage(img, 0, 0, width, height);

        // Detect faces
        setIsDetecting(true);
        setError(null);

        try {
          const detections = await faceapi.detectAllFaces(
            canvas,
            new faceapi.TinyFaceDetectorOptions({
              inputSize: 416,
              scoreThreshold: 0.5,
            })
          );

          const faces: FaceDetection[] = detections.map((d) => ({
            box: {
              x: d.box.x,
              y: d.box.y,
              width: d.box.width,
              height: d.box.height,
            },
            score: d.score,
          }));

          if (faces.length === 0) {
            setError(
              'No face detected in the image. Please try a different photo.'
            );
          } else if (faces.length > 1) {
            // Sort by size (largest first) and select the largest by default
            faces.sort(
              (a, b) => b.box.width * b.box.height - a.box.width * a.box.height
            );
          }

          setDetectedFaces(faces);
          setSelectedFaceIndex(0);
        } catch (detectErr) {
          const errMsg =
            detectErr instanceof Error ? detectErr.message : 'Unknown error';
          setError('Face detection failed: ' + errMsg);
        } finally {
          setIsDetecting(false);
        }
      };

      img.onerror = () => {
        setError('Failed to load image. Please try a different file.');
      };

      img.src = imageDataUrl;
    }, [imageDataUrl, isLoadingModels, modelError]);

    // Draw face detection box and update preview
    useEffect(() => {
      const canvas = originalCanvasRef.current;
      const previewCanvas = previewCanvasRef.current;
      const img = imageRef.current;
      if (!canvas || !img || detectedFaces.length === 0) return;

      const ctx = canvas.getContext('2d');
      if (!ctx) return;

      // Redraw original image
      ctx.drawImage(img, 0, 0, canvas.width, canvas.height);

      // Scale factor for detection boxes
      const scaleX = canvas.width / img.width;
      const scaleY = canvas.height / img.height;

      // Draw boxes for all faces
      detectedFaces.forEach((face, index) => {
        const isSelected = index === selectedFaceIndex;
        ctx.strokeStyle = isSelected ? '#22c55e' : '#94a3b8';
        ctx.lineWidth = isSelected ? 3 : 1;
        ctx.strokeRect(
          face.box.x * scaleX,
          face.box.y * scaleY,
          face.box.width * scaleX,
          face.box.height * scaleY
        );

        // Draw face number for multiple faces
        if (detectedFaces.length > 1) {
          ctx.fillStyle = isSelected ? '#22c55e' : '#94a3b8';
          ctx.font = '14px sans-serif';
          ctx.fillText(
            String(index + 1),
            face.box.x * scaleX + 4,
            face.box.y * scaleY + 16
          );
        }
      });

      // Update crop preview using helper function
      const selectedFace = detectedFaces[selectedFaceIndex];
      if (previewCanvas && selectedFace) {
        drawCropPreview(img, previewCanvas, selectedFace, padding);
      }
    }, [detectedFaces, selectedFaceIndex, padding]);

    // Handle save
    const handleSave = useCallback(() => {
      const img = imageRef.current;
      if (!img || detectedFaces.length === 0) return;

      const face = detectedFaces[selectedFaceIndex];
      if (!face) return;

      // Create crop canvas
      const cropCanvas = document.createElement('canvas');
      const cropSize = Math.max(face.box.width, face.box.height) + padding * 2;
      const cropX = face.box.x + face.box.width / 2 - cropSize / 2;
      const cropY = face.box.y + face.box.height / 2 - cropSize / 2;

      // Clamp to image bounds
      const clampedX = Math.max(0, Math.min(cropX, img.width - cropSize));
      const clampedY = Math.max(0, Math.min(cropY, img.height - cropSize));
      const clampedSize = Math.min(
        cropSize,
        img.width - clampedX,
        img.height - clampedY
      );

      cropCanvas.width = clampedSize;
      cropCanvas.height = clampedSize;

      const ctx = cropCanvas.getContext('2d');
      if (!ctx) return;

      ctx.drawImage(
        img,
        clampedX,
        clampedY,
        clampedSize,
        clampedSize,
        0,
        0,
        clampedSize,
        clampedSize
      );

      // Compress to final size
      const compressedDataUrl = compressImage(cropCanvas, 128, 0.9);

      modal.resolve({
        type: 'saved',
        imageDataUrl: compressedDataUrl,
      } as AvatarUploadResult);
      modal.hide();
    }, [detectedFaces, selectedFaceIndex, padding, modal]);

    const handleCancel = useCallback(() => {
      modal.resolve({ type: 'canceled' } as AvatarUploadResult);
      modal.hide();
    }, [modal]);

    const handleRetry = useCallback(() => {
      setImageDataUrl(null);
      setDetectedFaces([]);
      setError(null);
      if (fileInputRef.current) {
        fileInputRef.current.value = '';
      }
    }, []);

    const triggerFileInput = useCallback(() => {
      fileInputRef.current?.click();
    }, []);

    return (
      <Dialog open={modal.visible} onOpenChange={handleCancel}>
        <DialogHeader>
          <DialogTitle>Upload Avatar for {agentName}</DialogTitle>
          <DialogDescription>
            Select an image with a face. The face will be automatically detected
            and cropped.
          </DialogDescription>
        </DialogHeader>

        <DialogContent className="max-w-xl">
          {/* Loading models state */}
          {isLoadingModels && (
            <div className="flex flex-col items-center justify-center py-12 gap-4">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
              <p className="text-sm text-muted-foreground">
                Loading face detection models...
              </p>
            </div>
          )}

          {/* Model error state */}
          {modelError && (
            <Alert variant="destructive">
              <AlertTriangle className="h-4 w-4" />
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>{modelError}</AlertDescription>
            </Alert>
          )}

          {/* Main content when models loaded */}
          {!isLoadingModels && !modelError && (
            <div className="flex flex-col gap-6">
              {/* File input */}
              <input
                ref={fileInputRef}
                type="file"
                accept=".png,.jpg,.jpeg,image/png,image/jpeg"
                onChange={handleFileSelect}
                className="hidden"
              />

              {/* No image selected state */}
              {!imageDataUrl && (
                <div
                  onClick={triggerFileInput}
                  className="flex flex-col items-center justify-center py-12 gap-4 border-2 border-dashed border-muted-foreground/25 rounded-lg cursor-pointer hover:border-muted-foreground/50 transition-colors"
                >
                  {currentAvatar ? (
                    <img
                      src={currentAvatar}
                      alt="Current avatar"
                      className="w-20 h-20 rounded-full object-cover opacity-50"
                    />
                  ) : (
                    <User className="h-12 w-12 text-muted-foreground/50" />
                  )}
                  <div className="flex flex-col items-center gap-1">
                    <p className="text-sm font-medium">Click to select image</p>
                    <p className="text-xs text-muted-foreground">
                      PNG or JPEG, max 5MB
                    </p>
                  </div>
                  <Button variant="outline" size="sm">
                    <Upload className="h-4 w-4 mr-2" />
                    Choose File
                  </Button>
                </div>
              )}

              {/* Image loaded state */}
              {imageDataUrl && (
                <div className="flex flex-col gap-4">
                  {/* Error alert */}
                  {error && (
                    <Alert variant="destructive">
                      <AlertTriangle className="h-4 w-4" />
                      <AlertTitle>Detection Issue</AlertTitle>
                      <AlertDescription className="flex items-center justify-between">
                        <span>{error}</span>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={handleRetry}
                        >
                          <RefreshCw className="h-3 w-3 mr-1" />
                          Try Another
                        </Button>
                      </AlertDescription>
                    </Alert>
                  )}

                  {/* Detecting state */}
                  {isDetecting && (
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      <Loader2 className="h-4 w-4 animate-spin" />
                      Detecting faces...
                    </div>
                  )}

                  {/* Multiple faces warning */}
                  {detectedFaces.length > 1 && (
                    <Alert>
                      <ImageIcon className="h-4 w-4" />
                      <AlertTitle>Multiple Faces Detected</AlertTitle>
                      <AlertDescription>
                        {detectedFaces.length} faces found. Click a face number
                        or use buttons below to select.
                      </AlertDescription>
                    </Alert>
                  )}

                  {/* Canvas preview area */}
                  <div className="flex gap-6 items-start justify-center flex-wrap">
                    {/* Original with detection box */}
                    <div className="flex flex-col gap-2">
                      <Label className="text-xs text-muted-foreground">
                        Original
                      </Label>
                      <canvas
                        ref={originalCanvasRef}
                        className="border rounded max-w-[400px]"
                      />
                    </div>

                    {/* Crop preview */}
                    {detectedFaces.length > 0 && (
                      <div className="flex flex-col gap-2">
                        <Label className="text-xs text-muted-foreground">
                          Preview
                        </Label>
                        <canvas
                          ref={previewCanvasRef}
                          className="border rounded-full"
                          width={150}
                          height={150}
                        />
                      </div>
                    )}
                  </div>

                  {/* Face selection buttons for multiple faces */}
                  {detectedFaces.length > 1 && (
                    <div className="flex gap-2 items-center">
                      <Label className="text-sm">Select face:</Label>
                      <div className="flex gap-1">
                        {detectedFaces.map((_, index) => (
                          <Button
                            key={index}
                            variant={
                              index === selectedFaceIndex
                                ? 'default'
                                : 'outline'
                            }
                            size="sm"
                            onClick={() => setSelectedFaceIndex(index)}
                          >
                            {index + 1}
                          </Button>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Padding slider */}
                  {detectedFaces.length > 0 && (
                    <div className="flex flex-col gap-2">
                      <div className="flex justify-between items-center">
                        <Label className="text-sm">Padding around face</Label>
                        <span className="text-xs text-muted-foreground">
                          {padding}px
                        </span>
                      </div>
                      <Slider
                        value={[padding]}
                        onValueChange={(value) => setPadding(value[0])}
                        min={10}
                        max={50}
                        step={5}
                        className="w-full"
                      />
                    </div>
                  )}

                  {/* Action to select different file */}
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={triggerFileInput}
                    className="self-start"
                  >
                    <Upload className="h-3 w-3 mr-1" />
                    Choose Different Image
                  </Button>
                </div>
              )}
            </div>
          )}
        </DialogContent>

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={handleCancel}>
            Cancel
          </Button>
          <Button
            onClick={handleSave}
            disabled={
              isLoadingModels ||
              !!modelError ||
              detectedFaces.length === 0 ||
              isDetecting
            }
          >
            Save Avatar
          </Button>
        </DialogFooter>
      </Dialog>
    );
  }
);

export const AvatarUploadDialog = defineModal<
  AvatarUploadDialogProps,
  AvatarUploadResult
>(AvatarUploadDialogImpl);
