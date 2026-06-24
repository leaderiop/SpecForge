import { Text } from 'spectacle';
import { colors } from '../../theme/colors';

type CalloutVariant = 'warning' | 'success' | 'info' | 'danger';

const variantColors: Record<CalloutVariant, string> = {
  warning: colors.accent.yellow,
  success: colors.accent.green,
  info: colors.accent.blue,
  danger: colors.accent.red,
};

interface CalloutProps {
  variant: CalloutVariant;
  children: React.ReactNode;
}

export function Callout({ variant, children }: CalloutProps) {
  const color = variantColors[variant];
  return (
    <div style={{
      padding: '18px 28px',
      border: `2px solid ${color}`,
      borderRadius: 12,
      background: `${color}18`,
      margin: '24px 0',
      maxWidth: 1000,
      width: '100%',
    }}>
      <Text fontSize="24px" color={color} margin="0" fontWeight="bold">
        {children}
      </Text>
    </div>
  );
}
