import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  render() {
    if (this.state.hasError) {
      return (
        this.props.fallback ?? (
          <div className="h-full flex flex-col items-center justify-center gap-3 p-6 text-center">
            <p className="text-[var(--danger)] font-medium">Something went wrong</p>
            <p className="text-xs text-[var(--text-muted)] max-w-md">
              {this.state.error?.message}
            </p>
            <button
              onClick={() => this.setState({ hasError: false, error: null })}
              className="px-3 py-1.5 rounded bg-[var(--accent)] text-white text-xs hover:opacity-90 transition-opacity"
            >
              Retry
            </button>
          </div>
        )
      );
    }

    return this.props.children;
  }
}
