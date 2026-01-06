import { useState, useRef, useCallback, useEffect } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader } from '@/components/ui/loader';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { defineModal } from '@/lib/modals';
import {
  AlertTriangle,
  Upload,
  User,
  ZoomIn,
  ZoomOut,
  RefreshCw,
} from 'lucide-react';
import {
  loadFaceDetectionModels,
  detectFaces,
  getLargestFace,
  calculateFaceCropRegion,
  calculateCenterCropRegion,
  cropImage,
  compressImage,
  loadImageFromFile,
  isValidImageFile,
  isFileSizeValid,
  drawFaceOverlay,
  MAX_FILE_SIZE,
  type FaceDetectionResult,
  type CropRegion,
} from '@/lib/imageUtils';
import { cn } from '@/lib/utils';

export interface AvatarUploadDialogProps {
  /** Initial avatar image URL (for editing existing avatar) */
  initialAvatar?: string;
  /** Target size for the compressed image in KB */
  targetSizeKB?: number;
  /** Maximum output dimension in pixels */
  maxDimension?: number;
}

export type AvatarUploadResult =
  | { type: 'saved'; dataUrl: string }
  | { type: 'canceled' };

type DialogState =
  | 'idle'
  | 'loading-models'
  | 'loading-image'
  | 'detecting-faces'
  | 'ready'
  | 'saving';

const AvatarUploadDialogImpl = NiceModal.create<AvatarUploadDialogProps>(
  (props) => {
    const modal = useModal();
    const { targetSizeKB = 100, maxDimension = 512 } = props;

    // Refs
    const fileInputRef = useRef<HTMLInputElement>(null);
    const previewCanvasRef = useRef<HTMLCanvasElement>(null);
    const dialogRef = useRef<HTMLDivElement>(null);

    // State
    const [state, setState] = useState<DialogState>('idle');
    const [error, setError] = useState<string | null>(null);
    const [warning, setWarning] = useState<string | null>(null);
    const [sourceImage, setSourceImage] = useState<HTMLImageElement | null>(
      null
    );
    const [faces, setFaces] = useState<FaceDetectionResult[]>([]);
    const [cropRegion, setCropRegion] = useState<CropRegion | null>(null);
    const [paddingPercent, setPaddingPercent] = useState(50);
    const [previewDataUrl, setPreviewDataUrl] = useState<string | null>(null);

    // Load face detection models on mount
    useEffect(() => {
      const loadModels = async () => {
        setState('loading-models');
        try {
          await loadFaceDetectionModels();
          setState('idle');
        } catch (err) {
          setError(
            err instanceof Error
              ? err.message
              : 'Failed to load face detection models'
          );
          setState('idle');
        }
      };
      loadModels();
    }, []);

    // Update crop region when padding changes
    useEffect(() => {
      if (sourceImage && faces.length > 0) {
        const largestFace = getLargestFace(faces);
        if (largestFace) {
          const newCropRegion = calculateFaceCropRegion(
            largestFace,
            sourceImage.width,
            sourceImage.height,
            paddingPercent
          );
          setCropRegion(newCropRegion);
        }
      }
    }, [paddingPercent, sourceImage, faces]);

    // Update preview when crop region changes
    useEffect(() => {
      if (sourceImage && cropRegion) {
        updatePreview();
      }
    }, [sourceImage, cropRegion]);

    // Draw overlay on canvas when faces or crop region changes
    useEffect(() => {
      if (previewCanvasRef.current && sourceImage) {
        const canvas = previewCanvasRef.current;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        // Calculate display size (fit in preview area)
        const maxPreviewSize = 400;
        const scale = Math.min(
          maxPreviewSize / sourceImage.width,
          maxPreviewSize / sourceImage.height,
          1
        );

        canvas.width = sourceImage.width * scale;
        canvas.height = sourceImage.height * scale;

        // Draw image
        ctx.drawImage(sourceImage, 0, 0, canvas.width, canvas.height);

        // Draw overlay with scaled regions
        if (faces.length > 0 || cropRegion) {
          const scaledFaces = faces.map((f) => ({
            ...f,
            x: f.x * scale,
            y: f.y * scale,
            width: f.width * scale,
            height: f.height * scale,
          }));

          const scaledCrop = cropRegion
            ? {
                x: cropRegion.x * scale,
                y: cropRegion.y * scale,
                width: cropRegion.width * scale,
                height: cropRegion.height * scale,
              }
            : undefined;

          drawFaceOverlay(canvas, scaledFaces, scaledCrop);
        }
      }
    }, [sourceImage, faces, cropRegion]);

    const updatePreview = useCallback(() => {
      if (!sourceImage || !cropRegion) return;

      try {
        const croppedCanvas = cropImage(sourceImage, cropRegion);
        const compressed = compressImage(
          croppedCanvas,
          targetSizeKB,
          0.5,
          0.92,
          maxDimension
        );
        setPreviewDataUrl(compressed.dataUrl);
      } catch (err) {
        console.error('Failed to update preview:', err);
      }
    }, [sourceImage, cropRegion, targetSizeKB, maxDimension]);

    const handleFileSelect = useCallback(
      async (file: File) => {
        setError(null);
        setWarning(null);

        // Validate file type
        if (!isValidImageFile(file)) {
          setError(
            'Invalid file type. Please select a JPEG, PNG, GIF, WebP, or BMP image.'
          );
          return;
        }

        // Validate file size
        if (!isFileSizeValid(file)) {
          setError(
            `File is too large. Maximum size is ${Math.round(MAX_FILE_SIZE / (1024 * 1024))}MB.`
          );
          return;
        }

        setState('loading-image');

        try {
          const img = await loadImageFromFile(file);
          setSourceImage(img);

          setState('detecting-faces');

          const detectedFaces = await detectFaces(img);
          setFaces(detectedFaces);

          if (detectedFaces.length === 0) {
            setWarning(
              'No face detected. The image will be center-cropped. You can still save it or select a different image.'
            );
            const centerCrop = calculateCenterCropRegion(img.width, img.height);
            setCropRegion(centerCrop);
          } else {
            const largestFace = getLargestFace(detectedFaces);
            if (largestFace) {
              const faceCrop = calculateFaceCropRegion(
                largestFace,
                img.width,
                img.height,
                paddingPercent
              );
              setCropRegion(faceCrop);
            }

            if (detectedFaces.length > 1) {
              setWarning(
                `${detectedFaces.length} faces detected. Cropping to the largest face.`
              );
            }
          }

          setState('ready');
        } catch (err) {
          setError(
            err instanceof Error ? err.message : 'Failed to process image'
          );
          setState('idle');
        }
      },
      [paddingPercent]
    );

    const handleFileInputChange = useCallback(
      (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0];
        if (file) {
          handleFileSelect(file);
        }
        // Reset input so the same file can be selected again
        event.target.value = '';
      },
      [handleFileSelect]
    );

    const handleDrop = useCallback(
      (event: React.DragEvent) => {
        event.preventDefault();
        const file = event.dataTransfer.files[0];
        if (file) {
          handleFileSelect(file);
        }
      },
      [handleFileSelect]
    );

    const handleDragOver = useCallback((event: React.DragEvent) => {
      event.preventDefault();
    }, []);

    const handleSelectFile = useCallback(() => {
      fileInputRef.current?.click();
    }, []);

    const handleSave = useCallback(async () => {
      if (!sourceImage || !cropRegion) return;

      setState('saving');

      try {
        const croppedCanvas = cropImage(sourceImage, cropRegion);
        const compressed = compressImage(
          croppedCanvas,
          targetSizeKB,
          0.5,
          0.92,
          maxDimension
        );

        modal.resolve({
          type: 'saved',
          dataUrl: compressed.dataUrl,
        } as AvatarUploadResult);
        modal.hide();
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to save image');
        setState('ready');
      }
    }, [sourceImage, cropRegion, targetSizeKB, maxDimension, modal]);

    const handleCancel = useCallback(() => {
      modal.resolve({ type: 'canceled' } as AvatarUploadResult);
      modal.hide();
    }, [modal]);

    const handleReset = useCallback(() => {
      setSourceImage(null);
      setFaces([]);
      setCropRegion(null);
      setPreviewDataUrl(null);
      setError(null);
      setWarning(null);
      setState('idle');
    }, []);

    const handlePaddingChange = useCallback(
      (event: React.ChangeEvent<HTMLInputElement>) => {
        setPaddingPercent(Number(event.target.value));
      },
      []
    );

    const isLoading =
      state === 'loading-models' ||
      state === 'loading-image' ||
      state === 'detecting-faces' ||
      state === 'saving';

    const canSave = state === 'ready' && sourceImage && cropRegion;

    return (
      <Dialog
        ref={dialogRef}
        open={modal.visible}
        onOpenChange={(open) => !open && handleCancel()}
      >
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Upload Avatar</DialogTitle>
            <DialogDescription>
              Upload an image for your avatar. Face detection will automatically
              center the crop on your face.
            </DialogDescription>
          </DialogHeader>

          <div className="flex flex-col gap-4">
            {/* Error Alert */}
            {error && (
              <Alert variant="destructive">
                <AlertTriangle className="h-4 w-4" />
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            {/* Warning Alert */}
            {warning && (
              <Alert variant="default">
                <AlertTriangle className="h-4 w-4" />
                <AlertDescription>{warning}</AlertDescription>
              </Alert>
            )}

            {/* Loading State */}
            {isLoading && (
              <div className="flex items-center justify-center py-8">
                <Loader
                  message={
                    state === 'loading-models'
                      ? 'Loading face detection...'
                      : state === 'loading-image'
                        ? 'Loading image...'
                        : state === 'detecting-faces'
                          ? 'Detecting faces...'
                          : 'Saving...'
                  }
                />
              </div>
            )}

            {/* Idle State - File Selection */}
            {state === 'idle' && !isLoading && (
              <div
                className={cn(
                  'flex flex-col items-center justify-center gap-4 py-12 px-4',
                  'border-2 border-dashed rounded-lg',
                  'cursor-pointer hover:border-foreground/50 transition-colors'
                )}
                onClick={handleSelectFile}
                onDrop={handleDrop}
                onDragOver={handleDragOver}
              >
                <div className="p-4 rounded-full bg-muted">
                  <Upload className="h-8 w-8 text-muted-foreground" />
                </div>
                <div className="text-center">
                  <p className="text-sm font-medium">
                    Click to select or drag and drop
                  </p>
                  <p className="text-xs text-muted-foreground mt-1">
                    JPEG, PNG, GIF, WebP, or BMP (max 10MB)
                  </p>
                </div>
              </div>
            )}

            {/* Ready State - Preview and Crop */}
            {state === 'ready' && sourceImage && (
              <div className="flex flex-col md:flex-row gap-4">
                {/* Source Preview with Overlay */}
                <div className="flex-1">
                  <div className="text-sm font-medium mb-2">Source Image</div>
                  <div className="relative flex items-center justify-center bg-muted rounded-lg overflow-hidden">
                    <canvas
                      ref={previewCanvasRef}
                      className="max-w-full max-h-[300px]"
                    />
                  </div>

                  {/* Face Detection Badge */}
                  <div className="mt-2 flex items-center gap-2">
                    <User className="h-4 w-4 text-muted-foreground" />
                    <span className="text-xs text-muted-foreground">
                      {faces.length === 0
                        ? 'No faces detected'
                        : faces.length === 1
                          ? '1 face detected'
                          : `${faces.length} faces detected`}
                    </span>
                  </div>

                  {/* Padding Slider (only if face detected) */}
                  {faces.length > 0 && (
                    <div className="mt-4">
                      <div className="flex items-center justify-between mb-2">
                        <label className="text-sm font-medium">
                          Face Padding
                        </label>
                        <span className="text-xs text-muted-foreground">
                          {paddingPercent}%
                        </span>
                      </div>
                      <div className="flex items-center gap-2">
                        <ZoomOut className="h-4 w-4 text-muted-foreground" />
                        <input
                          type="range"
                          min="10"
                          max="150"
                          value={paddingPercent}
                          onChange={handlePaddingChange}
                          className="flex-1 h-2 bg-muted rounded-lg appearance-none cursor-pointer"
                        />
                        <ZoomIn className="h-4 w-4 text-muted-foreground" />
                      </div>
                    </div>
                  )}
                </div>

                {/* Cropped Preview */}
                <div className="flex-1">
                  <div className="text-sm font-medium mb-2">Preview</div>
                  <div className="flex items-center justify-center bg-muted rounded-lg p-4">
                    {previewDataUrl ? (
                      <img
                        src={previewDataUrl}
                        alt="Avatar preview"
                        className="w-32 h-32 rounded-full object-cover border-2 border-foreground/20"
                      />
                    ) : (
                      <div className="w-32 h-32 rounded-full bg-muted-foreground/20 flex items-center justify-center">
                        <User className="h-8 w-8 text-muted-foreground" />
                      </div>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground text-center mt-2">
                    This is how your avatar will appear
                  </p>
                </div>
              </div>
            )}

            {/* Hidden File Input */}
            <input
              ref={fileInputRef}
              type="file"
              accept="image/jpeg,image/png,image/gif,image/webp,image/bmp"
              onChange={handleFileInputChange}
              className="hidden"
            />
          </div>

          <DialogFooter className="gap-2 sm:gap-0">
            {state === 'ready' && (
              <Button
                variant="outline"
                onClick={handleReset}
                className="gap-2"
                type="button"
              >
                <RefreshCw className="h-4 w-4" />
                Choose Different
              </Button>
            )}
            <div className="flex-1" />
            <Button variant="outline" onClick={handleCancel} type="button">
              Cancel
            </Button>
            <Button
              onClick={handleSave}
              disabled={!canSave}
              type="submit"
              className="gap-2"
            >
              Save Avatar
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);

export const AvatarUploadDialog = defineModal<
  AvatarUploadDialogProps,
  AvatarUploadResult
>(AvatarUploadDialogImpl);
