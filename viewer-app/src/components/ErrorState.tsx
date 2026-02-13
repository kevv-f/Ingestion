/**
 * ErrorState component - Displays error messages with retry option
 * 
 * Requirements:
 * - 1.3: Display informative error message when database not found
 * - 1.4: Display error message with connection failure details
 * - 11.1: Display error message describing query failure
 * - 11.2: Display partial content message with available data
 */

import React from 'react';

export interface ErrorStateProps {
  /** Error message to display */
  message: string;
  /** Additional error details (optional) */
  details?: string;
  /** Callback for retry action (optional) */
  onRetry?: () => void;
  /** Custom retry button text (optional) */
  retryText?: string;
}

/**
 * ErrorState component
 * Displays an error message with optional details and retry button
 * 
 * @param message - Main error message to display
 * @param details - Additional error details
 * @param onRetry - Callback when retry button is clicked
 * @param retryText - Custom text for retry button
 */
export const ErrorState: React.FC<ErrorStateProps> = ({
  message,
  details,
  onRetry,
  retryText = 'Try Again',
}) => {
  return (
    <div className="error-state" data-testid="error-state" role="alert">
      {/* Error icon */}
      <div className="error-state__icon" aria-hidden="true">
        <svg
          width="48"
          height="48"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="8" x2="12" y2="12" />
          <line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
      </div>
      
      <h2 className="error-state__title">Something went wrong</h2>
      
      <p className="error-state__message" data-testid="error-message">
        {message}
      </p>
      
      {details && (
        <details className="error-state__details">
          <summary>Technical Details</summary>
          <pre data-testid="error-details">{details}</pre>
        </details>
      )}
      
      {onRetry && (
        <button
          className="error-state__retry"
          onClick={onRetry}
          data-testid="retry-button"
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <polyline points="23 4 23 10 17 10" />
            <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
          </svg>
          {retryText}
        </button>
      )}
    </div>
  );
};

export default ErrorState;
