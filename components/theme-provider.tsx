'use client';

import { createTheme, ThemeProvider as MuiThemeProvider } from '@mui/material/styles';
import { useTheme as useNextTheme } from 'next-themes';
import { ReactNode, useEffect, useState } from 'react';

interface ThemeProviderProps {
  children: ReactNode;
}

export default function ThemeProvider({ children }: ThemeProviderProps) {
  const { theme: nextTheme, systemTheme } = useNextTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  const currentTheme = nextTheme === 'system' ? systemTheme : nextTheme;

  const muiTheme = createTheme({
    palette: {
      mode: currentTheme === 'dark' ? 'dark' : 'light',
      background: {
        default: currentTheme === 'dark' ? '#0f1419' : '#ffffff',
        paper: currentTheme === 'dark' ? '#1a1f2e' : '#ffffff',
      },
      text: {
        primary: currentTheme === 'dark' ? '#ffffff' : '#000000',
        secondary: currentTheme === 'dark' ? '#b0b0b0' : '#666666',
      },
    },
  });

  if (!mounted) {
    return <>{children}</>;
  }

  return (
    <MuiThemeProvider theme={muiTheme}>
      {children}
    </MuiThemeProvider>
  );
} 