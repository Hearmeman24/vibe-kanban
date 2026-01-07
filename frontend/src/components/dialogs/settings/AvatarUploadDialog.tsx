import { useState, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2, Upload, X, ImageIcon } from 'lucide-react';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { defineModal } from '@/lib/modals';
import { useImageUpload } from '@/hooks/useImageUpload';

export interface AvatarUploadDialogProps {
  agentId: string;
  agentName: string;
}

export interface AvatarUploadResult {
  url: string;
  imageId: string;
}

const ALLOWED_TYPES = ['image/png', 'image/jpeg', 'image/webp'];
const MAX_FILE_SIZE = 5 * 1024 * 1024; // 5MB

const AvatarUploadDialogImpl = NiceModal.create<AvatarUploadDialogProps>(
  ({ agentName }) => {
    const modal = useModal();
    const { t } = useTranslation('settings');
    const { upload } = useImageUpload();
    const fileInputRef = useRef<HTMLInputElement>(null);

    const [selectedFile, setSelectedFile] = useState<File | null>(null);
    const [previewUrl, setPreviewUrl] = useState<string | null>(null);
    const [isUploading, setIsUploading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [isDragOver, setIsDragOver] = useState(false);

    const validateFile = useCallback(
      (file: File): string | null => {
        if (!ALLOWED_TYPES.includes(file.type)) {
          return t('settings.agentsConfiguration.upload.invalidFile');
        }
        if (file.size > MAX_FILE_SIZE) {
          return t('settings.agentsConfiguration.upload.fileTooLarge');
        }
        return null;
      },
      [t]
    );

    const handleFileSelect = useCallback(
      (file: File) => {
        const validationError = validateFile(file);
        if (validationError) {
          setError(validationError);
          return;
        }

        setError(null);
        setSelectedFile(file);

        // Create preview URL
        const url = URL.createObjectURL(file);
        setPreviewUrl(url);
      },
      [validateFile]
    );

    const handleInputChange = useCallback(
      (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (file) {
          handleFileSelect(file);
        }
      },
      [handleFileSelect]
    );

    const handleDragOver = useCallback((e: React.DragEvent) => {
      e.preventDefault();
      setIsDragOver(true);
    }, []);

    const handleDragLeave = useCallback((e: React.DragEvent) => {
      e.preventDefault();
      setIsDragOver(false);
    }, []);

    const handleDrop = useCallback(
      (e: React.DragEvent) => {
        e.preventDefault();
        setIsDragOver(false);

        const file = e.dataTransfer.files?.[0];
        if (file) {
          handleFileSelect(file);
        }
      },
      [handleFileSelect]
    );

    const handleUpload = async () => {
      if (!selectedFile) return;

      setIsUploading(true);
      setError(null);

      try {
        const result = await upload(selectedFile);
        modal.resolve({
          url: `/api/images/${result.id}`,
          imageId: result.id,
        } as AvatarUploadResult);
        modal.hide();
      } catch (err) {
        setError(
          err instanceof Error
            ? err.message
            : t('settings.agentsConfiguration.upload.error')
        );
        setIsUploading(false);
      }
    };

    const handleCancel = () => {
      // Clean up preview URL
      if (previewUrl) {
        URL.revokeObjectURL(previewUrl);
      }
      modal.resolve(null);
      modal.hide();
    };

    const handleOpenChange = (open: boolean) => {
      if (!open) {
        handleCancel();
      }
    };

    const handleClearSelection = () => {
      if (previewUrl) {
        URL.revokeObjectURL(previewUrl);
      }
      setSelectedFile(null);
      setPreviewUrl(null);
      setError(null);
      if (fileInputRef.current) {
        fileInputRef.current.value = '';
      }
    };

    const handleDropZoneClick = () => {
      fileInputRef.current?.click();
    };

    return (
      <Dialog open={modal.visible} onOpenChange={handleOpenChange}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>
              {t('settings.agentsConfiguration.upload.title')}
            </DialogTitle>
            <DialogDescription>
              {t('settings.agentsConfiguration.upload.formats')} - {agentName}
            </DialogDescription>
          </DialogHeader>

          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          <div className="space-y-4">
            {/* Hidden file input */}
            <input
              ref={fileInputRef}
              type="file"
              accept=".png,.jpg,.jpeg,.webp"
              onChange={handleInputChange}
              className="hidden"
            />

            {/* Drop zone / Preview */}
            {previewUrl ? (
              <div className="relative">
                <div className="flex justify-center p-4 border rounded-lg bg-muted/30">
                  <img
                    src={previewUrl}
                    alt="Preview"
                    className="max-h-48 max-w-full rounded-lg object-contain"
                  />
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  className="absolute top-2 right-2 h-6 w-6"
                  onClick={handleClearSelection}
                >
                  <X className="h-4 w-4" />
                </Button>
              </div>
            ) : (
              <div
                className={`
                  flex flex-col items-center justify-center p-8 border-2 border-dashed rounded-lg cursor-pointer
                  transition-colors
                  ${isDragOver ? 'border-primary bg-primary/5' : 'border-muted-foreground/25 hover:border-muted-foreground/50'}
                `}
                onDragOver={handleDragOver}
                onDragLeave={handleDragLeave}
                onDrop={handleDrop}
                onClick={handleDropZoneClick}
              >
                <ImageIcon className="h-10 w-10 text-muted-foreground mb-3" />
                <p className="text-sm text-muted-foreground text-center">
                  {t('settings.agentsConfiguration.upload.dragDrop')}
                </p>
                <p className="text-xs text-muted-foreground mt-1">
                  {t('settings.agentsConfiguration.upload.formats')}
                </p>
              </div>
            )}
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={handleCancel}
              disabled={isUploading}
            >
              {t('settings.agentsConfiguration.upload.cancel')}
            </Button>
            <Button
              onClick={handleUpload}
              disabled={!selectedFile || isUploading}
            >
              {isUploading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t('settings.agentsConfiguration.upload.uploading')}
                </>
              ) : (
                <>
                  <Upload className="mr-2 h-4 w-4" />
                  {t('settings.agentsConfiguration.upload.upload')}
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);

export const AvatarUploadDialog = defineModal<
  AvatarUploadDialogProps,
  AvatarUploadResult | null
>(AvatarUploadDialogImpl);
