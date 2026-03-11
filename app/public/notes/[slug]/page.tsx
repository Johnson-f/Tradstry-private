import { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { PublicNoteViewer } from './public-note-viewer';

interface PageProps {
  params: Promise<{ slug: string }>;
}

// Fetch note data for metadata
async function getPublicNote(slug: string) {
  try {
    const baseUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:9000';
    const res = await fetch(`${baseUrl}/api/notebook/collaboration/public/${slug}`, {
      cache: 'no-store',
    });
    
    if (!res.ok) return null;
    const data = await res.json();
    return data.data;
  } catch {
    return null;
  }
}

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { slug } = await params;
  const note = await getPublicNote(slug);
  
  if (!note) {
    return {
      title: 'Note Not Found | Tradstry',
    };
  }

  return {
    title: `${note.title} | Tradstry`,
    description: note.content_plain_text?.slice(0, 160) || 'A shared note from Tradstry',
    openGraph: {
      title: note.title,
      description: note.content_plain_text?.slice(0, 160) || 'A shared note from Tradstry',
      type: 'article',
    },
  };
}

export default async function PublicNotePage({ params }: PageProps) {
  const { slug } = await params;
  const note = await getPublicNote(slug);

  if (!note) {
    notFound();
  }

  return <PublicNoteViewer note={note} />;
}
