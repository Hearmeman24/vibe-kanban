import * as React from 'react';
import { cn } from '@/lib/utils';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';

export interface AgentAvatarDisplayProps {
  agentName: string;
  avatar?: string; // base64 data URL
  size?: 'sm' | 'md' | 'lg'; // 40px, 64px, 100px
  showName?: boolean;
  role?: string;
  onClick?: () => void;
}

const sizeMap = {
  sm: 40,
  md: 64,
  lg: 100,
} as const;

const textSizeMap = {
  sm: 'text-sm',
  md: 'text-lg',
  lg: 'text-2xl',
} as const;

export const AgentAvatarDisplay: React.FC<AgentAvatarDisplayProps> = ({
  agentName,
  avatar,
  size = 'md',
  showName = false,
  role,
  onClick,
}) => {
  const pixelSize = sizeMap[size];
  const textSize = textSizeMap[size];
  const initial = agentName.charAt(0).toUpperCase();

  const avatarElement = (
    <div
      className={cn(
        'relative flex items-center gap-3',
        onClick && 'cursor-pointer'
      )}
      onClick={onClick}
    >
      <div
        className={cn(
          'relative rounded-full overflow-hidden flex items-center justify-center bg-muted border border-border',
          onClick &&
            'hover:ring-2 hover:ring-ring hover:ring-offset-2 transition-all'
        )}
        style={{ width: pixelSize, height: pixelSize }}
      >
        {avatar ? (
          <img
            src={avatar}
            alt={`${agentName} avatar`}
            className="w-full h-full object-cover"
          />
        ) : (
          <span
            className={cn(
              'font-semibold text-muted-foreground select-none',
              textSize
            )}
          >
            {initial}
          </span>
        )}
      </div>
      {showName && (
        <div className="flex flex-col">
          <span className="font-medium text-foreground">{agentName}</span>
          {role && (
            <span className="text-xs text-muted-foreground">{role}</span>
          )}
        </div>
      )}
    </div>
  );

  // Wrap with tooltip when not showing name inline
  if (!showName) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>{avatarElement}</TooltipTrigger>
        <TooltipContent>
          <p className="font-medium">{agentName}</p>
          {role && <p className="text-xs text-muted-foreground">{role}</p>}
        </TooltipContent>
      </Tooltip>
    );
  }

  return avatarElement;
};

export default AgentAvatarDisplay;
