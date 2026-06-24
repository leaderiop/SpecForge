import { Component, ReactNode } from 'react';

interface Props {
  children: ReactNode;
  fallback?: ReactNode | ((error: Error) => ReactNode);
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error) {
    console.error('[ErrorBoundary]', error);
  }

  render() {
    if (this.state.hasError) {
      const { fallback } = this.props;
      if (typeof fallback === 'function') {
        return fallback(this.state.error!);
      }
      if (fallback) {
        return fallback;
      }
      return (
        <div style={{ padding: 40, color: '#ff6b6b', background: '#0f0f23', height: '100vh', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}>
          <h1 style={{ fontSize: 36, marginBottom: 16 }}>Slide Rendering Error</h1>
          <pre style={{ fontSize: 16, color: '#e2e8f0', maxWidth: '80%', overflow: 'auto' }}>
            {this.state.error?.message}
          </pre>
          <button
            onClick={() => this.setState({ hasError: false, error: null })}
            style={{ marginTop: 24, padding: '12px 24px', fontSize: 18, background: '#60a5fa', color: '#0f0f23', border: 'none', borderRadius: 8, cursor: 'pointer' }}
          >
            Try Again
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
