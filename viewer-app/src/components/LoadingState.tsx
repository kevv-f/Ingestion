/**
 * LoadingState component - Displays loading indicator
 * 
 * Requirement 10.4: THE Viewer_App SHALL display a loading indicator
 * while fetching content
 */

import React from 'react';

export interface LoadingStateProps {
  /** Custom loading message (optional) */
  message?: string;
  /** Size variant for the spinner (optional) */
  size?: 'sm' | 'md' | 'lg';
}

/**
 * Size mappings for spinner dimensions
 */
const SIZE_MAP = {
  sm: 24,
  md: 40,
  lg: 56,
} as const;

/**
 * LoadingState component
 * Displays a loading spinner with optional message
 * 
 * @param message - Custom loading message to display
 * @param size - Size variant for the spinner
 */
export const LoadingState: React.FC<LoadingStateProps> = ({
  message = 'Loading...',
  size = 'md',
}) => {
  const spinnerSize = SIZE_MAP[size];

  return (
    <div
      className="loading-state"
      data-testid="loading-state"
      role="status"
      aria-live="polite"
    >
      {/* Animated spinner */}
      <div
        className="loading-state__spinner"
        style={{ width: spinnerSize, height: spinnerSize }}
        aria-hidden="true"
      >
        <svg
          width={spinnerSize}
          height={spinnerSize}
          viewBox="0 0 24 24"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <circle
            className="loading-state__spinner-track"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="3"
            strokeOpacity="0.2"
          />
          <path
            className="loading-state__spinner-head"
            d="M12 2a10 10 0 0 1 10 10"
            stroke="currentColor"
            strokeWidth="3"
            strokeLinecap="round"
          >
            <animateTransform
              attributeName="transform"
              type="rotate"
              from="0 12 12"
              to="360 12 12"
              dur="1s"
              repeatCount="indefinite"
            />
          </path>
        </svg>
      </div>
      
      <p className="loading-state__message" data-testid="loading-message">
        {message}
      </p>
    </div>
  );
};

export default LoadingState;
