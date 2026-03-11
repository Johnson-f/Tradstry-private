import TradeTrackingClient from '@/app/app/journaling/tracking/[id]/tracking-client';

export const dynamic = 'force-dynamic';

export default function TradeTrackingPage({ params }: { params: Promise<{ id: string }> }) {
  return <TradeTrackingClient params={params} />;
}

