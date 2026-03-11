const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL || "http://127.0.0.1:9000";

// Helper function to build query strings from parameters
function buildQueryString(params: Record<string, string | number | boolean | undefined>): string {
  const queryParams = new URLSearchParams();
  
  Object.entries(params).forEach(([key, value]) => {
    if (value !== undefined) {
      queryParams.append(key, value.toString());
    }
  });
  
  const queryString = queryParams.toString();
  return queryString ? `?${queryString}` : '';
}

const endpoints = {
  baseURL: API_BASE_URL,
  apiPrefix: "/api",
  ws: {
    url: (token: string) => {
      // Use wss:// for HTTPS (production), ws:// for HTTP (development)
      const protocol = API_BASE_URL.startsWith('https') ? 'wss' : 'ws';
      const baseUrl = new URL(API_BASE_URL);
      return `${protocol}://${baseUrl.host}${apiConfig.apiPrefix}/ws?token=${encodeURIComponent(token)}`;
    },
    collab: (noteId: string, token: string) => {
      const protocol = API_BASE_URL.startsWith('https') ? 'wss' : 'ws';
      const baseUrl = new URL(API_BASE_URL);
      return `${protocol}://${baseUrl.host}${apiConfig.apiPrefix}/ws/collab/${noteId}?token=${encodeURIComponent(token)}`;
    },
  },
  endpoints: {
    // Root - unimportant endpoint 
    root: "/",
    health: "/health",
    profile: "/profile",
    me: "/me",
    myData: "/my-data",

    // Webhooks
    webhooks: {
      supabase: "/webhooks/supabase",
      clerk: "/webhooks/clerk",
    },

    // User management endpoints
    user: {
      initialize: "/user/initialize",
      check: (userId: string) => `/user/check/${userId}`,
      databaseInfo: (userId: string) => `/user/database-info/${userId}`,
      syncSchema: (userId: string) => `/user/sync-schema/${userId}`,
      schemaVersion: (userId: string) => `/user/schema-version/${userId}`,
      profile: (userId: string) => `/user/profile/${userId}`,
      profilePicture: (userId: string) => `/user/profile/picture/${userId}`,
      storage: "/user/storage",
      deleteAccount: "/user/account",
    },

    // Images endpoints
    images: {
      base: "/images",
      test: "/images/test",
      upload: "/images/upload",
      count: "/images/count",
      byTradeNote: (tradeNoteId: string) => `/images/trade-note/${tradeNoteId}`,
      byId: (imageId: string) => `/images/${imageId}`,
      url: (imageId: string) => `/images/${imageId}/url`,
    },

    // Stocks endpoints
    stocks: {
      base: "/stocks",
      test: "/stocks/test",
      count: "/stocks/count",
      byId: (id: number) => `/stocks/${id}`,
      /**
       * Build stocks endpoint with query parameters
       * @param params - Query parameters including openOnly filter
       * @returns Full endpoint URL with query string
       */
      withQuery: (params?: {
        openOnly?: boolean;
        symbol?: string;
        tradeType?: string;
        startDate?: string;
        endDate?: string;
        limit?: number;
        offset?: number;
        tradeGroupId?: string;
        parentTradeId?: number;
      }) => `/stocks${buildQueryString(params || {})}`,
      analytics: {
        summary: "/stocks/analytics",
        pnl: "/stocks/analytics/pnl",
        profitFactor: "/stocks/analytics/profit-factor",
        winRate: "/stocks/analytics/win-rate",
        lossRate: "/stocks/analytics/loss-rate",
        avgGain: "/stocks/analytics/avg-gain",
        avgLoss: "/stocks/analytics/avg-loss",
        biggestWinner: "/stocks/analytics/biggest-winner",
        biggestLoser: "/stocks/analytics/biggest-loser",
        avgHoldTimeWinners: "/stocks/analytics/avg-hold-time-winners",
        avgHoldTimeLosers: "/stocks/analytics/avg-hold-time-losers",
        riskRewardRatio: "/stocks/analytics/risk-reward-ratio",
        tradeExpectancy: "/stocks/analytics/trade-expectancy",
        avgPositionSize: "/stocks/analytics/avg-position-size",
        netPnl: "/stocks/analytics/net-pnl",
      },
    },

    // Options endpoints
    options: {
      base: "/options",
      test: "/options/test",
      count: "/options/count",
      byId: (id: number) => `/options/${id}`,
      /**
       * Build options endpoint with query parameters
       * @param params - Query parameters including openOnly filter
       * @returns Full endpoint URL with query string
       */
      withQuery: (params?: {
        openOnly?: boolean;
        symbol?: string;
        strategyType?: string;
        tradeDirection?: string;
        optionType?: string;
        status?: string;
        startDate?: string;
        endDate?: string;
        limit?: number;
        offset?: number;
        tradeGroupId?: string;
        parentTradeId?: number;
      }) => `/options${buildQueryString(params || {})}`,
      analytics: {
        summary: "/options/analytics",
        pnl: "/options/analytics/pnl",
        profitFactor: "/options/analytics/profit-factor",
        winRate: "/options/analytics/win-rate",
        lossRate: "/options/analytics/loss-rate",
        avgGain: "/options/analytics/avg-gain",
        avgLoss: "/options/analytics/avg-loss",
        biggestWinner: "/options/analytics/biggest-winner",
        biggestLoser: "/options/analytics/biggest-loser",
        avgHoldTimeWinners: "/options/analytics/avg-hold-time-winners",
        avgHoldTimeLosers: "/options/analytics/avg-hold-time-losers",
        riskRewardRatio: "/options/analytics/risk-reward-ratio",
        tradeExpectancy: "/options/analytics/trade-expectancy",
        avgPositionSize: "/options/analytics/avg-position-size",
        netPnl: "/options/analytics/net-pnl",
      },
    },

    // Trade Notes endpoints
    tradeNotes: {
      base: "/trade-notes",
      test: "/trade-notes/test",
      search: "/trade-notes/search",
      recent: "/trade-notes/recent",
      count: "/trade-notes/count",
      byId: (noteId: string) => `/trade-notes/${noteId}`,
      // Trade-linked notes
      byTrade: (tradeType: 'stock' | 'option', tradeId: number) => 
        `/trades/${tradeType}/${tradeId}/notes`,
    },

    // Trade Tags endpoints
    tradeTags: {
      base: "/trade-tags",
      categories: "/trade-tags/categories",
      byId: (id: string) => `/trade-tags/${id}`,
      // Trade associations
      stockTrade: (id: number) => `/trades/stock/${id}/tags`,
      optionTrade: (id: number) => `/trades/option/${id}/tags`,
    },

    // Playbook endpoints
    playbooks: {
      base: "/playbooks",
      test: "/playbooks/test",
      count: "/playbooks/count",
      byId: (id: string) => `/playbooks/${id}`,
      tag: "/playbooks/tag",
      untag: "/playbooks/untag",
      byTrade: (tradeId: number) => `/playbooks/trades/${tradeId}`,
      trades: (setupId: string) => `/playbooks/${setupId}/trades`,
      // Rules management
      rules: (id: string) => `/playbooks/${id}/rules`,
      rule: (id: string, ruleId: string) => `/playbooks/${id}/rules/${ruleId}`,
      // Missed trades
      missedTrades: (id: string) => `/playbooks/${id}/missed-trades`,
      missedTrade: (id: string, missedId: string) => `/playbooks/${id}/missed-trades/${missedId}`,
      // Analytics
      analytics: (id: string) => `/playbooks/${id}/analytics`,
      allAnalytics: "/playbooks/analytics",
    },

 

    // Notebook endpoints
    notebook: {
      // Notes
      notes: {
        base: "/notebook/notes",
        trash: "/notebook/notes/trash",
        byId: (id: string) => `/notebook/notes/${id}`,
        restore: (id: string) => `/notebook/notes/${id}/restore`,
        permanent: (id: string) => `/notebook/notes/${id}/permanent`,
        // Note tags
        tags: (noteId: string) => `/notebook/notes/${noteId}/tags`,
        tag: (noteId: string, tagId: string) => `/notebook/notes/${noteId}/tags/${tagId}`,
        // Note images
        images: (noteId: string) => `/notebook/notes/${noteId}/images`,
      },
      // Collaboration
      collaboration: {
        // Invitations
        invitations: {
          mine: "/notebook/collaboration/invitations",
          accept: "/notebook/collaboration/invitations/accept",
          decline: (id: string) => `/notebook/collaboration/invitations/${id}/decline`,
        },
        // Note-specific collaboration
        notes: {
          collaborators: (noteId: string) => `/notebook/collaboration/notes/${noteId}/collaborators`,
          invite: (noteId: string) => `/notebook/collaboration/notes/${noteId}/invite`,
          collaborator: (noteId: string, collabId: string) => `/notebook/collaboration/notes/${noteId}/collaborators/${collabId}`,
          // Sharing
          share: (noteId: string) => `/notebook/collaboration/notes/${noteId}/share`,
          visibility: (noteId: string) => `/notebook/collaboration/notes/${noteId}/visibility`,
          // Real-time collaboration
          join: (noteId: string) => `/notebook/collaboration/notes/${noteId}/join`,
          leave: (noteId: string) => `/notebook/collaboration/notes/${noteId}/leave`,
          cursor: (noteId: string) => `/notebook/collaboration/notes/${noteId}/cursor`,
          content: (noteId: string) => `/notebook/collaboration/notes/${noteId}/content`,
          participants: (noteId: string) => `/notebook/collaboration/notes/${noteId}/participants`,
        },
        // Public notes
        public: (slug: string) => `/notebook/collaboration/public/${slug}`,
      },
      // Folders
      folders: {
        base: "/notebook/folders",
        favorites: "/notebook/folders/favorites",
        byId: (id: string) => `/notebook/folders/${id}`,
      },
      // Tags
      tags: {
        base: "/notebook/tags",
        byId: (id: string) => `/notebook/tags/${id}`,
      },
      // Images
      images: {
        base: "/notebook/images",
        upload: "/notebook/images/upload",
        byId: (id: string) => `/notebook/images/${id}`,
      },
      // AI
      ai: {
        base: "/notebook/ai",
        process: "/notebook/ai/process",
      },
      // Calendar
      calendar: {
        // OAuth
        auth: {
          url: "/notebook/calendar/auth/url",
          callback: "/notebook/calendar/auth/callback",
        },
        // Connections
        connections: {
          base: "/notebook/calendar/connections",
          byId: (id: string) => `/notebook/calendar/connections/${id}`,
        },
        // Sync
        sync: {
          all: "/notebook/calendar/sync",
          byId: (id: string) => `/notebook/calendar/sync/${id}`,
        },
        // Events
        events: {
          base: "/notebook/calendar/events",
          byId: (id: string) => `/notebook/calendar/events/${id}`,
          withQuery: (params?: {
            start_time?: string;
            end_time?: string;
            source?: string;
            connection_id?: string;
            is_reminder?: boolean;
            limit?: number;
            offset?: number;
          }) => `/notebook/calendar/events${buildQueryString(params || {})}`,
        },
      },
    },

    // Analytics endpoints
    analytics: {
      // Core analytics engine endpoints (from analytics.rs)
      core: "/analytics/core",
      risk: "/analytics/risk",
      performance: "/analytics/performance",
      timeSeries: "/analytics/time-series",
      grouped: "/analytics/grouped",
      comprehensive: "/analytics/comprehensive",
      // Individual trade & symbol analytics (from core_metrics.rs)
      trade: "/analytics/trade",
      symbol: "/analytics/symbol",
    },

    // Market data endpoints
    market: {
      base: "/market",
      health: "/market/health",
      hours: "/market/hours",
      quotes: "/market/quotes",
      simpleQuotes: "/market/simple-quotes",
      similar: "/market/similar",
      logo: "/market/logo",
      detailedQuote: "/market/detailed-quote",
      historical: "/market/historical",
      movers: "/market/movers",
      gainers: "/market/gainers",
      losers: "/market/losers",
      actives: "/market/actives",
      news: "/market/news",
      indices: "/market/indices",
      sectors: "/market/sectors",
      search: "/market/search",
      indicators: "/market/indicators",
      financials: "/market/financials",
      earningsTranscript: "/market/earnings-transcript",
      earningsCalendar: "/market/earnings-calendar",
      holders: "/market/holders",
      subscribe: "/market/subscribe",
      unsubscribe: "/market/unsubscribe",
    },

    // AI endpoints
    ai: {
      chat: {
        base: "/ai/chat",
        send: "/ai/chat",
        stream: "/ai/chat/stream",
        sessions: {
          base: "/ai/chat/sessions",
          byId: (id: string) => `/ai/chat/sessions/${id}`,
          updateTitle: (id: string) => `/ai/chat/sessions/${id}/title`,
        },
      },
      insights: {
        base: "/ai/insights",
        generate: "/ai/insights",
        generateAsync: "/ai/insights/async",
        byId: (id: string) => `/ai/insights/${id}`,
        tasks: {
          byId: (taskId: string) => `/ai/insights/tasks/${taskId}`,
        },
      },
      reports: {
        base: "/reports",
        generate: "/reports",
        generateAsync: "/reports/async",
        list: "/reports", // GET endpoint for listing reports
        byId: (id: string) => `/reports/${id}`,
        tasks: {
          byId: (taskId: string) => `/reports/tasks/${taskId}`,
        },
      },
      analysis: {
        base: "/analysis",
        generate: "/analysis/generate",
      },
    },

    // Push notifications
    push: {
      base: "/push",
      subscribe: "/push/subscribe",
      unsubscribe: "/push/unsubscribe",
      test: "/push/test",
    },
    alerts: {
      base: "/alerts",
      subscribe: "/alerts/subscribe",
      unsubscribe: "/alerts/unsubscribe",
      test: "/alerts/test",
      rules: {
        base: "/alerts/rules",
        byId: (id: string) => `/alerts/rules/${id}`,
      },
    },
   
    // Brokerage endpoints
    brokerage: {
      base: "/brokerage",
      connections: {
        base: "/brokerage/connections",
        initiate: "/brokerage/connections/initiate",
        byId: (id: string) => `/brokerage/connections/${id}`,
        status: (id: string) => `/brokerage/connections/${id}/status`,
        complete: (id: string) => `/brokerage/connections/${id}/complete`,
      },
      accounts: {
        base: "/brokerage/accounts",
        sync: "/brokerage/accounts/sync",
        detail: (id: string) => `/brokerage/accounts/${id}/detail`,
        positions: (id: string) => `/brokerage/accounts/${id}/positions`,
        optionPositions: (id: string) => `/brokerage/accounts/${id}/positions/options`,
        transactions: (id: string) => `/brokerage/accounts/${id}/transactions`,
      },
      transactions: {
        base: "/brokerage/transactions",
        merge: "/brokerage/transactions/merge",
      },
      holdings: {
        base: "/brokerage/holdings",
      },
      unmatched: {
        base: "/brokerage/unmatched-transactions",
        byId: (id: string) => `/brokerage/unmatched-transactions/${id}`,
        resolve: (id: string) => `/brokerage/unmatched-transactions/${id}/resolve`,
        ignore: (id: string) => `/brokerage/unmatched-transactions/${id}/ignore`,
        suggestions: (id: string) => `/brokerage/unmatched-transactions/${id}/suggestions`,
      },
    },
   
  },
} as const;

// Type definitions for API configuration
export interface ApiConfig {
  baseURL: string;
  apiPrefix: string;
  ws: {
    url: (token: string) => string;
    collab: (noteId: string, token: string) => string;
  };
  endpoints: typeof endpoints.endpoints;
  timeout: number;
  retries: number;
}

export const apiConfig: ApiConfig = {
  baseURL: API_BASE_URL,
  apiPrefix: "/api",
  ws: {
    url: (token: string) => {
      const protocol = API_BASE_URL.startsWith('https') ? 'wss' : 'ws';
      const baseUrl = new URL(API_BASE_URL);
      return `${protocol}://${baseUrl.host}${apiConfig.apiPrefix}/ws?token=${encodeURIComponent(token)}`;
    },
    collab: (noteId: string, token: string) => {
      const protocol = API_BASE_URL.startsWith('https') ? 'wss' : 'ws';
      const baseUrl = new URL(API_BASE_URL);
      return `${protocol}://${baseUrl.host}${apiConfig.apiPrefix}/ws/collab/${noteId}?token=${encodeURIComponent(token)}`;
    },
  },
  endpoints: endpoints.endpoints,
  timeout: 30000, // 30 seconds (comment was incorrect - 30000ms = 30s not 5 min)
  retries: 3,
};

export const getFullUrl = (endpoint: string): string => {
  return `${apiConfig.baseURL}${apiConfig.apiPrefix}${endpoint}`;
};

export default apiConfig;
