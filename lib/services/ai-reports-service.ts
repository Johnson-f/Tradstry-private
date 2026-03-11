import { apiConfig, getFullUrl } from '@/lib/config/api';
import { createClient } from '@/lib/supabase/client';
import type {
  ApiResponse,
  ReportRequest,
  SaveReportPayload,
  TradingReport,
} from '@/lib/types/ai-reports';
import { AIReportsError } from '@/lib/types/ai-reports';

// Normalize a generated TradingReport into the exact payload the backend expects
// for /api/reports/save (avoids serialization/typing mismatches that cause 400s).
export const normalizeReportForSave = (report: TradingReport): SaveReportPayload => ({
  time_range: report.time_range,
  title: report.title,
  summary: report.summary ?? '',
  analytics: {
    total_pnl: Number(report.analytics.total_pnl ?? 0),
    win_rate: Number(report.analytics.win_rate ?? 0),
    profit_factor: Number(report.analytics.profit_factor ?? 0),
    avg_gain: Number(report.analytics.avg_gain ?? 0),
    avg_loss: Number(report.analytics.avg_loss ?? 0),
    biggest_winner: Number(report.analytics.biggest_winner ?? 0),
    biggest_loser: Number(report.analytics.biggest_loser ?? 0),
    avg_hold_time_winners: Number(report.analytics.avg_hold_time_winners ?? 0),
    avg_hold_time_losers: Number(report.analytics.avg_hold_time_losers ?? 0),
    risk_reward_ratio: Number(report.analytics.risk_reward_ratio ?? 0),
    trade_expectancy: Number(report.analytics.trade_expectancy ?? 0),
    avg_position_size: Number(report.analytics.avg_position_size ?? 0),
    net_pnl: Number(report.analytics.net_pnl ?? 0),
    total_trades: Number(report.analytics.total_trades ?? 0),
    winning_trades: Number(report.analytics.winning_trades ?? 0),
    losing_trades: Number(report.analytics.losing_trades ?? 0),
    break_even_trades: Number(report.analytics.break_even_trades ?? 0),
  },
  trades: report.trades.map((trade) => ({
    ...trade,
    quantity: Number(trade.quantity ?? 0),
    entry_price: Number(trade.entry_price ?? 0),
    exit_price:
      trade.exit_price === undefined || trade.exit_price === null
        ? undefined
        : Number(trade.exit_price),
    pnl:
      trade.pnl === undefined || trade.pnl === null ? undefined : Number(trade.pnl),
    entry_date: new Date(trade.entry_date).toISOString(),
    exit_date: trade.exit_date ? new Date(trade.exit_date).toISOString() : undefined,
  })),
  recommendations: Array.isArray(report.recommendations)
    ? [...report.recommendations]
    : [],
  risk_metrics: report.risk_metrics
    ? {
        max_drawdown: Number(report.risk_metrics.max_drawdown ?? 0),
        sharpe_ratio: Number(report.risk_metrics.sharpe_ratio ?? 0),
        volatility: Number(report.risk_metrics.volatility ?? 0),
        var_95: Number(report.risk_metrics.var_95 ?? 0),
        var_99: Number(report.risk_metrics.var_99 ?? 0),
        risk_score: Number(report.risk_metrics.risk_score ?? 0),
        concentration_risk: Number(report.risk_metrics.concentration_risk ?? 0),
        leverage_risk: Number(report.risk_metrics.leverage_risk ?? 0),
      }
    : undefined,
  performance_metrics: report.performance_metrics
    ? {
        monthly_returns: report.performance_metrics.monthly_returns.map((v) => Number(v ?? 0)),
        quarterly_returns: report.performance_metrics.quarterly_returns.map((v) => Number(v ?? 0)),
        yearly_returns: report.performance_metrics.yearly_returns.map((v) => Number(v ?? 0)),
        best_month: Number(report.performance_metrics.best_month ?? 0),
        worst_month: Number(report.performance_metrics.worst_month ?? 0),
        consistency_score: Number(report.performance_metrics.consistency_score ?? 0),
        trend_direction: report.performance_metrics.trend_direction ?? 'neutral',
        momentum_score: Number(report.performance_metrics.momentum_score ?? 0),
      }
    : undefined,
  metadata: {
    ...report.metadata,
    trade_count: Number(report.metadata.trade_count ?? 0),
    analysis_period_days: Number(report.metadata.analysis_period_days ?? 0),
    processing_time_ms: Number(report.metadata.processing_time_ms ?? 0),
    data_quality_score: Number(report.metadata.data_quality_score ?? 0),
    charts_generated: Number(report.metadata.charts_generated ?? 0),
  },
});

// Minimal AI Reports client: trigger WS streaming, save payloads, fetch history.
class AIReportsService {
  private supabase = createClient();

  private async getAuthToken(): Promise<string | null> {
    const { data: { session }, error } = await this.supabase.auth.getSession();
    if (error || !session?.access_token) return null;
    return session.access_token;
  }

  private endpoint(path: 'generate' | 'save' | 'history' | 'historyById', id?: string) {
    const base = apiConfig.endpoints.ai.reports?.base ?? '/api/reports';
    switch (path) {
      case 'generate':
        return getFullUrl(`${base}/generate`);
      case 'save':
        return getFullUrl(`${base}/save`);
      case 'history':
        return getFullUrl(`${base}/history`);
      case 'historyById':
        return getFullUrl(`${base}/history/${id}`);
    }
  }

  // Kick off generation (stream comes via WS) and return generated reports as fallback
  async generateReport(request: ReportRequest): Promise<TradingReport[]> {
    const token = await this.getAuthToken();
    if (!token) throw new AIReportsError('Authentication required', 'AUTH_ERROR');

    const res = await fetch(this.endpoint('generate'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify(request),
    });

    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      throw new AIReportsError(
        err.message || `HTTP ${res.status}: ${res.statusText}`,
        'GENERATION_ERROR',
      );
    }

    const json: ApiResponse<TradingReport[]> = await res.json();
    if (!json.success || !json.data) {
      throw new AIReportsError('Failed to generate reports', 'GENERATION_ERROR');
    }
    return json.data;
  }

  // Save streamed report payload
  async saveReport(payload: SaveReportPayload): Promise<string> {
    const token = await this.getAuthToken();
    if (!token) throw new AIReportsError('Authentication required', 'AUTH_ERROR');

    const res = await fetch(this.endpoint('save'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify(payload),
    });

    if (!res.ok) {
      const raw = await res.text().catch(() => '');
      const parsed = (() => {
        try {
          return raw ? JSON.parse(raw) : {};
        } catch {
          return {};
        }
      })();
      const message = parsed.message || parsed.error || raw;
      throw new AIReportsError(
        message || `HTTP ${res.status}: ${res.statusText}`,
        'SAVE_ERROR',
      );
    }

    const json: ApiResponse<{ id: string }> = await res.json();
    if (!json.success || !json.data?.id) {
      throw new AIReportsError('Failed to save report', 'SAVE_ERROR');
    }
    return json.data.id;
  }

  // Fetch history (list)
  async getHistory(limit = 50, offset = 0): Promise<import('@/lib/types/ai-reports').ReportHistoryResponse> {
      const token = await this.getAuthToken();
    if (!token) throw new AIReportsError('Authentication required', 'AUTH_ERROR');

    const url = new URL(this.endpoint('history'));
    url.searchParams.set('limit', String(limit));
    url.searchParams.set('offset', String(offset));

    const res = await fetch(url.toString(), {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      throw new AIReportsError(
        err.message || `HTTP ${res.status}: ${res.statusText}`,
        'FETCH_ERROR',
      );
    }

    const json: ApiResponse<import('@/lib/types/ai-reports').ReportHistoryResponse> = await res.json();
    if (!json.success || !json.data) {
      throw new AIReportsError('Failed to fetch history', 'FETCH_ERROR');
    }
    return json.data;
  }

  // Fetch single report from history
  async getHistoryById(id: string): Promise<TradingReport | unknown> {
      const token = await this.getAuthToken();
    if (!token) throw new AIReportsError('Authentication required', 'AUTH_ERROR');

    const res = await fetch(this.endpoint('historyById', id), {
      headers: { Authorization: `Bearer ${token}` },
    });

    if (res.status === 404) {
      throw new AIReportsError('Report not found', 'NOT_FOUND');
    }

    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      throw new AIReportsError(
        err.message || `HTTP ${res.status}: ${res.statusText}`,
        'FETCH_ERROR',
      );
    }

    const json: ApiResponse<TradingReport | unknown> = await res.json();
    if (!json.success || !json.data) {
      throw new AIReportsError('Failed to fetch report', 'FETCH_ERROR');
    }
    return json.data;
  }

}

export const aiReportsService = new AIReportsService();