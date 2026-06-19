import { Component, type ReactNode } from 'react';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error?: Error;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: { componentStack?: string }) {
    console.error('ErrorBoundary caught:', error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex items-center justify-center h-dvh p-8">
          <div className="max-w-md text-center space-y-4">
            <h1 className="text-xl font-semibold text-destructive">
              Something went wrong
            </h1>
            <p className="text-muted-foreground text-sm">
              {this.state.error?.message || 'An unexpected error occurred.'}
            </p>
            <button
              className="text-sm text-primary underline"
              onClick={() =>
                this.setState({ hasError: false, error: undefined })
              }
            >
              Try again
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
