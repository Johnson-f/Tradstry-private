import { NextResponse } from 'next/server';

export async function GET() {
  try {
    // Basic health check for Next.js frontend
    const health = {
      status: 'healthy',
      service: 'frontend',
      timestamp: new Date().toISOString(),
      version: '1.0.0',
      environment: process.env.NODE_ENV || 'development',
    };

    return NextResponse.json(health, { status: 200 });
  } catch (error) {
    const errorHealth = {
      status: 'unhealthy',
      service: 'frontend',
      timestamp: new Date().toISOString(),
      error: error instanceof Error ? error.message : 'Unknown error',
    };

    return NextResponse.json(errorHealth, { status: 500 });
  }
}
