import { MetadataRoute } from 'next';

export default function manifest(): MetadataRoute.Manifest {
  const icon192 = '/icons/icon-192.png';
  const icon512 = '/icons/icon-512.png';

  return {
    name: 'Tradstry',
    short_name: 'Tradstry',
    description: 'Trading journal, alerts, and analytics platform.',
    start_url: '/',
    display: 'standalone',
    background_color: '#000000',
    theme_color: '#000000',
    icons: [
      {
        src: icon192,
        sizes: '192x192',
        type: 'image/png',
        purpose: 'any',
      },
      {
        src: icon512,
        sizes: '512x512',
        type: 'image/png',
        purpose: 'any',
      },
    ],
  };
}

