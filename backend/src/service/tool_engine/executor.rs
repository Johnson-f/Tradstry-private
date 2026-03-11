use crate::service::ai_service::model_connection::openrouter::ToolCall;
use crate::service::market_engine::{
    analysts, calendar, earnings_transcripts, financials, historical, holders, hours, indices,
    movers, news, quotes, search, sectors,
};
use anyhow::{Context, Result};
use serde_json::{self, Value};

/// Tool Engine that executes tool calls by routing to market_engine functions
pub struct ToolEngine;

impl ToolEngine {
    pub fn new() -> Self {
        Self
    }

    /// Execute a tool call and return the result as JSON
    pub async fn execute(&self, tool_call: &ToolCall) -> Result<Value> {
        let function_name = &tool_call.function.name;
        let args_str = &tool_call.function.arguments;

        log::info!(
            "ToolEngine: Executing tool '{}' with args: {}",
            function_name,
            args_str
        );

        // Parse arguments JSON
        let args: Value = serde_json::from_str(args_str).context(format!(
            "Failed to parse tool arguments for {}",
            function_name
        ))?;

        let result = match function_name.as_str() {
            "get_stock_quote" => {
                let symbols: Vec<String> = args["symbols"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("symbols must be an array"))?
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();

                if symbols.is_empty() {
                    return Err(anyhow::anyhow!("symbols array cannot be empty"));
                }

                let _symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
                let quotes = quotes::get_quotes(&symbols).await?;
                serde_json::to_value(quotes)?
            }

            "get_financials" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let statement = args["statement"].as_str().map(|s| s.to_string());
                let frequency = args["frequency"].as_str().map(|s| s.to_string());

                let financials =
                    financials::get_financials(&symbol, statement.as_deref(), frequency.as_deref())
                        .await?;

                serde_json::to_value(financials)?
            }

            "get_market_news" => {
                let symbol = args.get("symbol").and_then(|v| v.as_str());
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as u32);

                let news_items = news::get_news(symbol, limit).await?;
                serde_json::to_value(news_items)?
            }

            "get_earnings_transcript" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let quarter = args.get("quarter").and_then(|v| v.as_str());
                let year = args.get("year").and_then(|v| v.as_i64()).map(|n| n as i32);

                let transcript =
                    earnings_transcripts::get_earnings_transcript(&symbol, quarter, year).await?;

                serde_json::to_value(transcript)?
            }

            "search_symbol" => {
                let query = args["query"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("query is required"))?
                    .to_string();

                let hits = args.get("hits").and_then(|v| v.as_u64()).map(|n| n as u32);

                let results = search::search(&query, hits, None).await?;
                serde_json::to_value(results)?
            }

            "get_historical_data" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let range = args.get("range").and_then(|v| v.as_str());
                let interval = args.get("interval").and_then(|v| v.as_str());

                let historical = historical::get_historical(&symbol, range, interval).await?;

                serde_json::to_value(historical)?
            }

            // Phase 1: Calendar Tools (Earnings via Supabase Edge Function)
            "get_earnings_calendar" | "get_full_calendar" => {
                let from_date = args.get("from_date").and_then(|v| v.as_str());
                let to_date = args.get("to_date").and_then(|v| v.as_str());
                let include_logos = args
                    .get("include_logos")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let earnings =
                    calendar::fetch_earnings_calendar(from_date, to_date, include_logos).await?;
                serde_json::to_value(earnings)?
            }

            // Phase 2: Holders Tools
            "get_major_holders" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let holders = holders::get_major_holders(&symbol).await?;
                serde_json::to_value(holders)?
            }

            "get_institutional_holders" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let institutional = holders::get_institutional_holders(&symbol).await?;
                serde_json::to_value(institutional)?
            }

            "get_mutual_fund_holders" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let mutual_funds = holders::get_mutual_fund_holders(&symbol).await?;
                serde_json::to_value(mutual_funds)?
            }

            "get_insider_transactions" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                // Note: limit parameter is not supported by the underlying function
                // The function returns all transactions
                let transactions = holders::get_insider_transactions(&symbol).await?;
                serde_json::to_value(transactions)?
            }

            "get_insider_purchases" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let purchases = holders::get_insider_purchases(&symbol).await?;
                serde_json::to_value(purchases)?
            }

            "get_insider_roster" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let roster = holders::get_insider_roster(&symbol).await?;
                serde_json::to_value(roster)?
            }

            "get_all_holders" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let all_holders = holders::get_all_holders(&symbol).await?;
                serde_json::to_value(all_holders)?
            }

            "get_custom_holders" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let holder_type_str = args["holder_type"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("holder_type is required"))?;

                // Convert string to HolderType enum
                let holder_type = match holder_type_str {
                    "major" => holders::HolderType::Major,
                    "institutional" => holders::HolderType::Institutional,
                    "mutual_fund" => holders::HolderType::MutualFund,
                    "insider_transactions" => holders::HolderType::InsiderTransactions,
                    "insider_purchases" => holders::HolderType::InsiderPurchases,
                    "insider_roster" => holders::HolderType::InsiderRoster,
                    _ => return Err(anyhow::anyhow!("Invalid holder_type: {}", holder_type_str)),
                };

                // get_custom_holders expects a slice of HolderType
                let custom_holders = holders::get_custom_holders(&symbol, &[holder_type]).await?;
                serde_json::to_value(custom_holders)?
            }

            // Phase 3: Analysts Tools
            "get_recommendations" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let recommendations = analysts::get_recommendations(&symbol).await?;
                serde_json::to_value(recommendations)?
            }

            "get_upgrades_downgrades" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let upgrades_downgrades = analysts::get_upgrades_downgrades(&symbol).await?;
                serde_json::to_value(upgrades_downgrades)?
            }

            "get_price_targets" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let price_targets = analysts::get_price_targets(&symbol).await?;
                serde_json::to_value(price_targets)?
            }

            "get_earnings_history" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let earnings_history = analysts::get_earnings_history(&symbol).await?;
                serde_json::to_value(earnings_history)?
            }

            "get_earnings_estimates" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let earnings_estimates = analysts::get_earnings_estimates(&symbol).await?;
                serde_json::to_value(earnings_estimates)?
            }

            "get_revenue_estimates" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let revenue_estimates = analysts::get_revenue_estimates(&symbol).await?;
                serde_json::to_value(revenue_estimates)?
            }

            "get_eps_trend" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let eps_trend = analysts::get_eps_trend(&symbol).await?;
                serde_json::to_value(eps_trend)?
            }

            "get_eps_revisions" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let eps_revisions = analysts::get_eps_revisions(&symbol).await?;
                serde_json::to_value(eps_revisions)?
            }

            "get_growth_estimates" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let growth_estimates = analysts::get_growth_estimates(&symbol).await?;
                serde_json::to_value(growth_estimates)?
            }

            "get_all_analyst_data" => {
                let symbol = args["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
                    .to_string();

                let all_analyst_data = analysts::get_all_analyst_data(&symbol).await?;
                serde_json::to_value(all_analyst_data)?
            }

            // Phase 4: Sectors Tools
            "get_all_sectors" => {
                let sectors_list = sectors::get_all_sectors();
                serde_json::to_value(sectors_list)?
            }

            "get_sector_performance" => {
                let sector_str = args["sector"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("sector is required"))?
                    .to_lowercase();

                // Convert string to Sector enum
                use finance_query_core::Sector;
                let sector = match sector_str.as_str() {
                    "technology" => Sector::Technology,
                    "healthcare" => Sector::Healthcare,
                    "financial" | "financial_services" => Sector::FinancialServices,
                    "consumer_cyclical" => Sector::ConsumerCyclical,
                    "communication" | "communication_services" => Sector::Communication,
                    "industrials" => Sector::Industrials,
                    "consumer_defensive" => Sector::ConsumerDefensive,
                    "energy" => Sector::Energy,
                    "utilities" => Sector::Utilities,
                    "real_estate" => Sector::RealEstate,
                    "basic_materials" => Sector::BasicMaterials,
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Invalid sector: {}. Use get_all_sectors to see available sectors.",
                            sector_str
                        ));
                    }
                };

                let performance = sectors::get_sector_performance(sector).await?;
                serde_json::to_value(performance)?
            }

            "get_all_sectors_performance" => {
                let performance = sectors::get_all_sectors_performance().await?;
                serde_json::to_value(performance)?
            }

            "get_sectors" => {
                let sectors_overview = sectors::get_sectors().await?;
                serde_json::to_value(sectors_overview)?
            }

            "get_sector_top_companies" => {
                let sector_str = args["sector"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("sector is required"))?
                    .to_lowercase();

                // Convert string to Sector enum
                use finance_query_core::Sector;
                let sector = match sector_str.as_str() {
                    "technology" => Sector::Technology,
                    "healthcare" => Sector::Healthcare,
                    "financial" | "financial_services" => Sector::FinancialServices,
                    "consumer_cyclical" => Sector::ConsumerCyclical,
                    "communication" | "communication_services" => Sector::Communication,
                    "industrials" => Sector::Industrials,
                    "consumer_defensive" => Sector::ConsumerDefensive,
                    "energy" => Sector::Energy,
                    "utilities" => Sector::Utilities,
                    "real_estate" => Sector::RealEstate,
                    "basic_materials" => Sector::BasicMaterials,
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Invalid sector: {}. Use get_all_sectors to see available sectors.",
                            sector_str
                        ));
                    }
                };

                // Default count to 25 if not provided
                let count = args
                    .get("count")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(25);

                let top_companies = sectors::get_sector_top_companies(sector, count).await?;
                serde_json::to_value(top_companies)?
            }

            "get_industry" => {
                let industry_key = args["industry_key"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("industry_key is required"))?
                    .to_string();

                let industry = sectors::get_industry(&industry_key).await?;
                serde_json::to_value(industry)?
            }

            // Phase 5: Market Movers Tools
            "get_movers" => {
                let movers_data = movers::get_movers().await?;
                serde_json::to_value(movers_data)?
            }

            "get_gainers" => {
                let count = args.get("count").and_then(|v| v.as_u64()).map(|n| n as u32);
                let gainers = movers::get_gainers(count).await?;
                serde_json::to_value(gainers)?
            }

            "get_losers" => {
                let count = args.get("count").and_then(|v| v.as_u64()).map(|n| n as u32);
                let losers = movers::get_losers(count).await?;
                serde_json::to_value(losers)?
            }

            "get_most_active" => {
                let count = args.get("count").and_then(|v| v.as_u64()).map(|n| n as u32);
                let most_active = movers::get_most_active(count).await?;
                serde_json::to_value(most_active)?
            }

            // Phase 6: Indices & Hours Tools
            "get_indices" => {
                let indices_data = indices::get_indices().await?;
                serde_json::to_value(indices_data)?
            }

            "get_market_hours" => {
                let market_hours = hours::get_hours().await?;
                serde_json::to_value(market_hours)?
            }

            _ => {
                return Err(anyhow::anyhow!("Unknown tool: {}", function_name));
            }
        };

        log::info!("ToolEngine: Successfully executed tool '{}'", function_name);
        Ok(result)
    }
}
