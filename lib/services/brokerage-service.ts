import { apiConfig, getFullUrl } from '@/lib/config/api';
import { createClient } from '@/lib/supabase/client';
import type {
  ConnectBrokerageRequest,
  ConnectBrokerageResponse,
  BrokerageConnection,
  BrokerageAccount,
  BrokerageTransaction,
  BrokerageHolding,
  SyncSummary,
  ConnectionStatusResponse,
  GetTransactionsQuery,
  GetHoldingsQuery,
  GetAccountTransactionsQuery,
  UnmatchedTransaction,
  ResolveUnmatchedRequest,
  ResolveUnmatchedResponse,
  UnmatchedSuggestion,
  AccountDetailResponse,
  AccountPositionsResponse,
  ApiResponse,
  MergeTransactionsRequest,
} from '@/lib/types/brokerage';

/**
 * Brokerage Service
 * Handles all brokerage API calls to the Rust backend
 */
export class BrokerageService {
  private baseURL: string;
  private supabase;

  constructor() {
    this.baseURL = apiConfig.baseURL + apiConfig.apiPrefix;
    this.supabase = createClient();
  }

  /**
   * Get authentication token
   */
  private async getAuthToken(): Promise<string | null> {
    try {
      const { data: { session } } = await this.supabase.auth.getSession();
      return session?.access_token || null;
    } catch (error) {
      console.error('Error getting auth token:', error);
      return null;
    }
  }

  /**
   * Initiate brokerage connection
   */
  async initiateConnection(
    request: ConnectBrokerageRequest
  ): Promise<ConnectBrokerageResponse> {
    const url = getFullUrl(apiConfig.endpoints.brokerage.connections.initiate);
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to initiate connection: ${response.statusText}`
      );
    }

    const result: ApiResponse<ConnectBrokerageResponse> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to initiate connection');
    }

    return result.data;
  }

  /**
   * List all brokerage connections
   */
  async listConnections(): Promise<BrokerageConnection[]> {
    const url = getFullUrl(apiConfig.endpoints.brokerage.connections.base);
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to list connections: ${response.statusText}`
      );
    }

    const result: ApiResponse<BrokerageConnection[]> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to list connections');
    }

    return result.data;
  }

  /**
   * Get connection status
   */
  async getConnectionStatus(
    connectionId: string
  ): Promise<ConnectionStatusResponse> {
    const url = getFullUrl(
      apiConfig.endpoints.brokerage.connections.status(connectionId)
    );
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to get connection status: ${response.statusText}`
      );
    }

    const result: ApiResponse<ConnectionStatusResponse> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to get connection status');
    }

    return result.data;
  }

  /**
   * Delete brokerage connection
   */
  async deleteConnection(connectionId: string): Promise<void> {
    const url = getFullUrl(
      apiConfig.endpoints.brokerage.connections.byId(connectionId)
    );
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'DELETE',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to delete connection: ${response.statusText}`
      );
    }

    const result: ApiResponse<unknown> = await response.json();

    if (!result.success) {
      throw new Error(result.message || 'Failed to delete connection');
    }
  }

  /**
   * List all brokerage accounts
   */
  async listAccounts(): Promise<BrokerageAccount[]> {
    const url = getFullUrl(apiConfig.endpoints.brokerage.accounts.base);
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to list accounts: ${response.statusText}`
      );
    }

    const result: ApiResponse<BrokerageAccount[]> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to list accounts');
    }

    return result.data;
  }

  /**
   * Sync accounts from brokerage
   */
  async syncAccounts(): Promise<SyncSummary> {
    const url = getFullUrl(apiConfig.endpoints.brokerage.accounts.sync);
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to sync accounts: ${response.statusText}`
      );
    }

    const result: ApiResponse<SyncSummary> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to sync accounts');
    }

    return result.data;
  }

  /**
   * Get transactions
   */
  async getTransactions(
    query?: GetTransactionsQuery & { exclude_transformed?: boolean }
  ): Promise<BrokerageTransaction[]> {
    const url = new URL(
      getFullUrl(apiConfig.endpoints.brokerage.transactions.base)
    );

    if (query?.account_id) {
      url.searchParams.append('account_id', query.account_id);
    }

    if (query?.exclude_transformed !== undefined) {
      url.searchParams.append('exclude_transformed', String(query.exclude_transformed));
    }

    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url.toString(), {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to get transactions: ${response.statusText}`
      );
    }

    const result: ApiResponse<BrokerageTransaction[]> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to get transactions');
    }

    return result.data;
  }

  /**
   * Merge transactions into a stock or option trade
   */
  async mergeTransactions(
    request: MergeTransactionsRequest
  ): Promise<unknown> {
    const url = getFullUrl(apiConfig.endpoints.brokerage.transactions.merge);
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to merge transactions: ${response.statusText}`
      );
    }

    const result: ApiResponse<unknown> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to merge transactions');
    }

    return result.data;
  }

  /**
   * Get holdings
   */
  async getHoldings(query?: GetHoldingsQuery): Promise<BrokerageHolding[]> {
    const url = new URL(
      getFullUrl(apiConfig.endpoints.brokerage.holdings.base)
    );

    if (query?.account_id) {
      url.searchParams.append('account_id', query.account_id);
    }

    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url.toString(), {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to get holdings: ${response.statusText}`
      );
    }

    const result: ApiResponse<BrokerageHolding[]> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to get holdings');
    }

    return result.data;
  }

  /**
   * Complete connection sync - fetches all accounts, positions, and transactions
   */
  async completeConnectionSync(connectionId: string): Promise<SyncSummary> {
    const url = getFullUrl(
      apiConfig.endpoints.brokerage.connections.complete(connectionId)
    );
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to complete connection sync: ${response.statusText}`
      );
    }

    const result: ApiResponse<SyncSummary> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to complete connection sync');
    }

    return result.data;
  }

  /**
   * Get account detail
   */
  async getAccountDetail(accountId: string): Promise<AccountDetailResponse> {
    const url = getFullUrl(
      apiConfig.endpoints.brokerage.accounts.detail(accountId)
    );
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to get account detail: ${response.statusText}`
      );
    }

    const result: ApiResponse<AccountDetailResponse> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to get account detail');
    }

    return result.data;
  }

  /**
   * Get account positions
   */
  async getAccountPositions(accountId: string): Promise<AccountPositionsResponse> {
    const url = getFullUrl(
      apiConfig.endpoints.brokerage.accounts.positions(accountId)
    );
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to get account positions: ${response.statusText}`
      );
    }

    const result: ApiResponse<AccountPositionsResponse> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to get account positions');
    }

    return result.data;
  }

  /**
   * Get account option positions
   */
  async getAccountOptionPositions(accountId: string): Promise<AccountPositionsResponse> {
    const url = getFullUrl(
      apiConfig.endpoints.brokerage.accounts.optionPositions(accountId)
    );
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to get account option positions: ${response.statusText}`
      );
    }

    const result: ApiResponse<AccountPositionsResponse> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to get account option positions');
    }

    return result.data;
  }

  /**
   * Get account transactions
   */
  async getAccountTransactions(
    accountId: string,
    query?: GetAccountTransactionsQuery
  ): Promise<unknown> {
    const url = new URL(
      getFullUrl(apiConfig.endpoints.brokerage.accounts.transactions(accountId))
    );

    if (query?.start_date) {
      url.searchParams.append('start_date', query.start_date);
    }
    if (query?.end_date) {
      url.searchParams.append('end_date', query.end_date);
    }
    if (query?.offset !== undefined) {
      url.searchParams.append('offset', query.offset.toString());
    }
    if (query?.limit !== undefined) {
      url.searchParams.append('limit', query.limit.toString());
    }

    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url.toString(), {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to get account transactions: ${response.statusText}`
      );
    }

    const result: ApiResponse<unknown> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to get account transactions');
    }

    return result.data;
  }

  /**
   * Get unmatched transactions
   */
  async getUnmatchedTransactions(): Promise<UnmatchedTransaction[]> {
    const url = getFullUrl(
      apiConfig.endpoints.brokerage.unmatched.base
    );
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to get unmatched transactions: ${response.statusText}`
      );
    }

    const result: ApiResponse<UnmatchedTransaction[]> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to get unmatched transactions');
    }

    return result.data;
  }

  /**
   * Resolve unmatched transaction
   */
  async resolveUnmatchedTransaction(
    id: string,
    request: ResolveUnmatchedRequest
  ): Promise<ResolveUnmatchedResponse> {
    const url = getFullUrl(
        apiConfig.endpoints.brokerage.unmatched.resolve(id)
    );
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to resolve unmatched transaction: ${response.statusText}`
      );
    }

    const result: ApiResponse<ResolveUnmatchedResponse> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to resolve unmatched transaction');
    }

    return result.data;
  }

  /**
   * Ignore unmatched transaction
   */
  async ignoreUnmatchedTransaction(id: string): Promise<void> {
    const url = getFullUrl(
      apiConfig.endpoints.brokerage.unmatched.ignore(id)
    );
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to ignore unmatched transaction: ${response.statusText}`
      );
    }

    const result: ApiResponse<unknown> = await response.json();

    if (!result.success) {
      throw new Error(result.message || 'Failed to ignore unmatched transaction');
    }
  }

  /**
   * Get unmatched transaction suggestions
   */
  async getUnmatchedSuggestions(id: string): Promise<UnmatchedSuggestion[]> {
    const url = getFullUrl(
      apiConfig.endpoints.brokerage.unmatched.suggestions(id)
    );
    const token = await this.getAuthToken();

    if (!token) {
      throw new Error('Authentication required');
    }

    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        (errorData as ApiResponse<unknown>).message ||
          `Failed to get unmatched suggestions: ${response.statusText}`
      );
    }

    const result: ApiResponse<UnmatchedSuggestion[]> = await response.json();

    if (!result.success || !result.data) {
      throw new Error(result.message || 'Failed to get unmatched suggestions');
    }

    return result.data;
  }
}

// Export singleton instance
export const brokerageService = new BrokerageService();