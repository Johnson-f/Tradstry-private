"use client"

import { useEffect, useMemo, useRef, useState } from "react"
import * as d3 from "d3"
import { useAnalyticsCore, useAnalyticsTimeSeries } from "@/lib/hooks/use-analytics"
import type { AnalyticsRequest, TimeSeriesData as AnalyticsTimeSeriesData, TimeSeriesPoint as AnalyticsTimeSeriesPoint } from "@/lib/types/analytics"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Info } from "lucide-react"

type TimeRangeOption = "7d" | "30d" | "90d" | "1y" | "ytd" | "all_time"

interface BasicAnalyticsProps {
  initialTimeRange?: TimeRangeOption
}

export default function BasicAnalytics({ initialTimeRange = "30d" }: BasicAnalyticsProps) {
  const svgRef = useRef<SVGSVGElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const tooltipRef = useRef<HTMLDivElement>(null)
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 })

  // Build analytics request from props
  const request: AnalyticsRequest = useMemo(() => ({
    time_range: initialTimeRange,
    options: {
      include_time_series: true,
    }
  }), [initialTimeRange])

  // Fetch core metrics (totals, averages, ratios)
  const { data: coreMetrics, isLoading: isLoadingCore, error: coreError } = useAnalyticsCore(request)

  // Fetch time series for the net P&L chart
  const { data: timeSeries, isLoading: isLoadingTs, error: tsError } = useAnalyticsTimeSeries(request)

  // Setup responsive dimensions
  useEffect(() => {
    const updateDimensions = () => {
      if (containerRef.current) {
        setDimensions({
          width: containerRef.current.offsetWidth,
          height: 60,
        })
      }
    }

    updateDimensions()
    window.addEventListener("resize", updateDimensions)
    return () => window.removeEventListener("resize", updateDimensions)
  }, [])

  // Map core metrics into the view model used by the cards
  const combinedMetrics = useMemo(() => {
    if (!coreMetrics) return null
    
    // Debug log to verify profit_factor connection
    console.debug('[BasicAnalytics] Processing Core Metrics:', {
      profit_factor: coreMetrics.profit_factor,
      win_rate: coreMetrics.win_rate,
      total_pnl: coreMetrics.total_pnl,
      total_trades: coreMetrics.total_trades,
    })
    
    return {
      total_pnl: coreMetrics.total_pnl,
      profit_factor: coreMetrics.profit_factor,
      win_rate: coreMetrics.win_rate,
      winning_trades: coreMetrics.winning_trades,
      losing_trades: coreMetrics.losing_trades,
      average_win: coreMetrics.average_win,
      average_loss: coreMetrics.average_loss,
    }
  }, [coreMetrics])

  // Use API time series for the net P&L chart
   // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
  const timeSeriesData: AnalyticsTimeSeriesData = useMemo(() => ({
    daily_pnl: timeSeries?.daily_pnl ?? [],
    weekly_pnl: timeSeries?.weekly_pnl ?? [],
    monthly_pnl: timeSeries?.monthly_pnl ?? [],
    rolling_win_rate: timeSeries?.rolling_win_rate ?? [],
    rolling_sharpe_ratio: timeSeries?.rolling_sharpe_ratio ?? [],
    profit_by_day_of_week: timeSeries?.profit_by_day_of_week ?? {},
    profit_by_month: timeSeries?.profit_by_month ?? {},
    drawdown_curve: timeSeries?.drawdown_curve ?? [],
  }), [timeSeries])

  // Render D3 chart
  useEffect(() => {
    if (
      !timeSeriesData?.daily_pnl ||
      timeSeriesData.daily_pnl.length === 0 ||
      !svgRef.current ||
      dimensions.width === 0
    ) {
      return
    }

    const dailyPnl = timeSeriesData.daily_pnl

    // Clear previous chart
    d3.select(svgRef.current).selectAll("*").remove()

    const svg = d3.select(svgRef.current)
    const margin = { top: 4, right: 6, bottom: 4, left: 6 }
    const width = dimensions.width - margin.left - margin.right
    const height = dimensions.height - margin.top - margin.bottom

    // Create scales
    const xScale = d3
      .scaleTime()
      .domain(d3.extent(dailyPnl, (d) => new Date(d.date)) as [Date, Date])
      .range([margin.left, width])

    const yDomain = d3.extent(dailyPnl, (d) => d.cumulative_value) as [number, number]
    const yScale = d3
      .scaleLinear()
      .domain(yDomain)
      .nice()
      .range([height - margin.bottom, margin.top])

    const defs = svg.append("defs")
    const gradient = defs
      .append("linearGradient")
      .attr("id", "pnl-gradient")
      .attr("x1", "0%")
      .attr("y1", "0%")
      .attr("x2", "0%")
      .attr("y2", "100%")

    gradient.append("stop").attr("offset", "0%").attr("stop-color", "rgba(45, 212, 191, 0.3)")
    gradient.append("stop").attr("offset", "100%").attr("stop-color", "rgba(45, 212, 191, 0.05)")

    // Create area generator
    const area = d3
      .area<AnalyticsTimeSeriesPoint>()
      .x((d) => xScale(new Date(d.date)))
      .y0(yScale(Math.max(0, Math.min(...yDomain))))
      .y1((d) => yScale(d.cumulative_value))
      .curve(d3.curveMonotoneX)

    // Create line generator
    const line = d3
      .line<AnalyticsTimeSeriesPoint>()
      .x((d) => xScale(new Date(d.date)))
      .y((d) => yScale(d.cumulative_value))
      .curve(d3.curveMonotoneX)

    // Draw area
    svg.append("path").datum(dailyPnl).attr("fill", "url(#pnl-gradient)").attr("d", area)

    svg
      .append("path")
      .datum(dailyPnl)
      .attr("fill", "none")
      .attr("stroke", "#2dd4bf")
      .attr("stroke-width", 2)
      .attr("d", line)

    // Add interactive dots for hover
    const dots = svg
      .selectAll(".dot")
      .data(dailyPnl)
      .enter()
      .append("circle")
      .attr("class", "dot")
      .attr("cx", (d) => xScale(new Date(d.date)))
      .attr("cy", (d) => yScale(d.cumulative_value))
      .attr("r", 0)
      .attr("fill", "#0891b2")
      .attr("stroke", "#fff")
      .attr("stroke-width", 2)

    // Add hover interactions
    dots
      .on("mouseenter", function (event, d) {
        d3.select(this).attr("r", 5)

        if (tooltipRef.current) {
          tooltipRef.current.style.opacity = "1"
          tooltipRef.current.style.left = `${event.pageX + 10}px`
          tooltipRef.current.style.top = `${event.pageY - 10}px`

          const date = new Date(d.date).toLocaleDateString("en-US", {
            month: "short",
            day: "numeric",
            year: "numeric",
          })

          tooltipRef.current.innerHTML = `
            <div class="text-xs space-y-1">
              <div class="font-semibold">${date}</div>
              <div>Daily P&L: <span class="font-medium">$${d.value.toFixed(2)}</span></div>
              <div>Cumulative P&L: <span class="font-medium">$${d.cumulative_value.toFixed(2)}</span></div>
              <div>Trades: <span class="font-medium">${d.trade_count}</span></div>
            </div>
          `
        }
      })
      .on("mousemove", (event) => {
        if (tooltipRef.current) {
          tooltipRef.current.style.left = `${event.pageX + 10}px`
          tooltipRef.current.style.top = `${event.pageY - 10}px`
        }
      })
      .on("mouseleave", function () {
        d3.select(this).attr("r", 0)
        if (tooltipRef.current) {
          tooltipRef.current.style.opacity = "0"
        }
      })
  }, [timeSeriesData, dimensions.width, dimensions.height])

  if (coreError || tsError) {
    return (
      <Card>
        <CardContent className="pt-4">
          <p className="text-red-600">Error loading analytics: {(coreError || tsError)?.message}</p>
        </CardContent>
      </Card>
    )
  }

  if (isLoadingCore || isLoadingTs) {
    return (
      <div className="flex gap-2 overflow-x-auto items-stretch">
        {/* Skeleton: Net Cumulative P&L card */}
        <Card className="flex-1 min-w-[230px] bg-white min-h-[138px] animate-pulse">
          <CardHeader className="pb-1">
            <div className="flex justify-between items-start">
              <div>
                <div className="flex items-center gap-2">
                  <div className="h-4 bg-gray-200 rounded w-32"></div>
                  <div className="h-4 bg-gray-100 rounded w-10"></div>
                </div>
                <div className="h-7 bg-gray-200 rounded mt-1 w-28"></div>
              </div>
            </div>
          </CardHeader>
          <CardContent className="pt-0 pb-1">
            <div className="w-full">
              <div className="h-[60px] bg-gray-100 rounded"></div>
            </div>
          </CardContent>
        </Card>

        {/* Skeleton: Profit Factor */}
        <Card className="flex-1 min-w-[170px] bg-white min-h-[138px] animate-pulse">
          <CardHeader className="pb-1">
            <div className="h-4 bg-gray-200 rounded w-24"></div>
          </CardHeader>
          <CardContent className="flex items-center justify-between pt-0 pb-1">
            <div className="h-7 bg-gray-200 rounded w-16"></div>
            <div className="h-9 bg-gray-100 rounded w-24"></div>
          </CardContent>
        </Card>

        {/* Skeleton: Win Rate */}
        <Card className="flex-1 min-w-[170px] bg-white min-h-[138px] animate-pulse">
          <CardHeader className="pb-1">
            <div className="h-4 bg-gray-200 rounded w-24"></div>
          </CardHeader>
          <CardContent className="flex items-center justify-between pt-0 pb-1">
            <div className="h-7 bg-gray-200 rounded w-20"></div>
            <div className="h-9 bg-gray-100 rounded w-24"></div>
          </CardContent>
        </Card>

        {/* Skeleton: Avg Win/Loss */}
        <Card className="flex-1 min-w-[170px] bg-white min-h-[138px] animate-pulse">
          <CardHeader className="pb-1">
            <div className="h-4 bg-gray-200 rounded w-28"></div>
          </CardHeader>
          <CardContent className="pt-0 pb-1">
            <div className="flex items-center gap-2">
              <div className="h-7 bg-gray-200 rounded w-16"></div>
              <div className="h-9 bg-gray-100 rounded w-28"></div>
            </div>
          </CardContent>
        </Card>
      </div>
    )
  }

  // Check if we have no data at all
  const hasNoData = !coreMetrics || coreMetrics.total_trades === 0
  
  if (hasNoData) {
    return (
      <div className="flex gap-2 overflow-x-auto items-stretch">
        <Card className="flex-1 min-w-[230px] bg-white min-h-[138px]">
          <CardHeader className="pb-1">
            <CardTitle className="text-xs font-normal text-gray-600">Net Cumulative P&L</CardTitle>
          </CardHeader>
          <CardContent className="pt-0 pb-1 flex items-center justify-center">
            <p className="text-sm text-muted-foreground">No trade data available for the selected period</p>
          </CardContent>
        </Card>

        <Card className="flex-1 min-w-[170px] bg-white min-h-[138px]">
          <CardHeader className="pb-1">
            <CardTitle className="text-xs font-normal text-gray-600">Profit Factor</CardTitle>
          </CardHeader>
          <CardContent className="pt-0 pb-1 flex items-center justify-center">
            <p className="text-2xl font-bold text-gray-400">-</p>
          </CardContent>
        </Card>

        <Card className="flex-1 min-w-[170px] bg-white min-h-[138px]">
          <CardHeader className="pb-1">
            <CardTitle className="text-xs font-normal text-gray-600">Trade Win %</CardTitle>
          </CardHeader>
          <CardContent className="pt-0 pb-1 flex items-center justify-center">
            <p className="text-2xl font-bold text-gray-400">-</p>
          </CardContent>
        </Card>

        <Card className="flex-1 min-w-[170px] bg-white min-h-[138px]">
          <CardHeader className="pb-1">
            <CardTitle className="text-xs font-normal text-gray-600">Avg win/loss trade</CardTitle>
          </CardHeader>
          <CardContent className="pt-0 pb-1 flex items-center justify-center">
            <p className="text-2xl font-bold text-gray-400">-</p>
          </CardContent>
        </Card>
      </div>
    )
  }

  const currentPnl = timeSeriesData.daily_pnl[timeSeriesData.daily_pnl.length - 1]?.cumulative_value || 0
  const dataPointCount = timeSeriesData.daily_pnl.length

  return (
    <>
      {/* Tooltip */}
      <div
        ref={tooltipRef}
        className="absolute bg-gray-900 text-white p-2 rounded shadow-lg pointer-events-none z-50 opacity-0 transition-opacity"
        style={{ fontSize: "12px" }}
      />

      <div className="flex gap-2 overflow-x-auto items-stretch">
        {/* Card 1: Net Cumulative P&L */}
        <Card className="flex-1 min-w-[230px] bg-white min-h-[138px]">
          <CardHeader className="pb-1">
            <div className="flex justify-between items-start">
              <div>
                <div className="flex items-center gap-2">
                  <CardTitle className="text-xs font-normal text-gray-600">Net Cumulative P&L</CardTitle>
                  <CardDescription className="text-xs font-medium text-gray-900 bg-gray-100 px-2 py-0.5 rounded">
                    {dataPointCount}
                  </CardDescription>
                </div>
                <div className="text-2xl font-bold text-gray-900 mt-1">${currentPnl.toFixed(2)}</div>
              </div>
            </div>
          </CardHeader>
          <CardContent className="pt-0 pb-1">
            <div ref={containerRef} className="w-full">
              <svg
                ref={svgRef}
                width={dimensions.width}
                height={dimensions.height}
                className="w-full"
                style={{ overflow: "visible" }}
              />
            </div>
          </CardContent>
        </Card>

        {/* Card 2: Profit Factor */}
        <div className="flex-1 min-w-[170px]">
          <ProfitFactorCard profitFactor={combinedMetrics?.profit_factor} />
        </div>

        {/* Card 3: Win Rate */}
        <div className="flex-1 min-w-[170px]">
          <WinRateCard
            winRate={combinedMetrics?.win_rate}
            losingTrades={combinedMetrics?.losing_trades}
            winningTrades={combinedMetrics?.winning_trades}
          />
        </div>

        {/* Card 4: Avg Win/Loss */}
        <div className="flex-1 min-w-[170px]">
          <AvgWinLossCard averageWin={combinedMetrics?.average_win} averageLoss={combinedMetrics?.average_loss} />
        </div>
      </div>
    </>
  )
}

function ProfitFactorCard({ profitFactor }: { profitFactor: number | null | undefined }) {
  const gaugeRef = useRef<SVGSVGElement>(null)

  useEffect(() => {
    if (!gaugeRef.current || profitFactor == null || !isFinite(profitFactor)) return

    const width = 90
    const height = 36
    const radius = 25

    const svg = d3.select(gaugeRef.current)
    svg.selectAll("*").remove()

    const g = svg.append("g").attr("transform", `translate(${width / 2}, ${height - 5})`)

    // Background arc
    const backgroundArc = d3
      .arc()
      .innerRadius(radius - 7)
      .outerRadius(radius)
      .startAngle(-Math.PI / 2)
      .endAngle(Math.PI / 2)

    g.append("path")
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      .attr("d", backgroundArc as any)
      .attr("fill", "#e5e7eb")

    // Foreground arc (red for profit factor)
    const normalizedValue = Math.min(profitFactor / 5, 1) // Normalize to 0-1, max at 5
    const foregroundArc = d3
      .arc()
      .innerRadius(radius - 7)
      .outerRadius(radius)
      .startAngle(-Math.PI / 2)
      .endAngle(-Math.PI / 2 + Math.PI * normalizedValue)

    g.append("path")
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      .attr("d", foregroundArc as any)
      .attr("fill", "#ef4444")
  }, [profitFactor])

  // Format display value (handle Infinity case)
  const displayValue = useMemo(() => {
    if (profitFactor == null) return 'N/A'
    if (!isFinite(profitFactor)) return '∞'
    return profitFactor.toFixed(2)
  }, [profitFactor])

  return (
    <Card className="bg-white min-h-[138px]">
      <CardHeader className="pb-1">
        <div className="flex items-center gap-1.5">
          <CardTitle className="text-xs font-normal text-gray-600">Profit Factor</CardTitle>
          <Info className="w-3 h-3 text-gray-400" />
        </div>
      </CardHeader>
      <CardContent className="flex items-center justify-between pt-0 pb-1">
        <div className="text-2xl font-bold text-gray-900">{displayValue}</div>
        <svg ref={gaugeRef} width={90} height={36} />
      </CardContent>
    </Card>
  )
}

function WinRateCard({
  winRate,
}: {
  winRate: number | null | undefined
  losingTrades?: number | null | undefined
  winningTrades?: number | null | undefined
}) {
  const gaugeRef = useRef<SVGSVGElement>(null)

  useEffect(() => {
    if (!gaugeRef.current || winRate == null) return

    const width = 90
    const height = 36
    const radius = 25

    const svg = d3.select(gaugeRef.current)
    svg.selectAll("*").remove()

    const g = svg.append("g").attr("transform", `translate(${width / 2}, ${height - 5})`)

    const winRateValue = winRate > 1 ? winRate / 100 : winRate

    // Create gradient for gauge
    const defs = svg.append("defs")
    const gradient = defs
      .append("linearGradient")
      .attr("id", "gauge-gradient")
      .attr("x1", "0%")
      .attr("y1", "0%")
      .attr("x2", "100%")
      .attr("y2", "0%")

    gradient.append("stop").attr("offset", "0%").attr("stop-color", "#10b981")
    gradient.append("stop").attr("offset", "50%").attr("stop-color", "#3b82f6")
    gradient.append("stop").attr("offset", "100%").attr("stop-color", "#ef4444")

    // Background arc
    const backgroundArc = d3
      .arc()
      .innerRadius(radius - 7)
      .outerRadius(radius)
      .startAngle(-Math.PI / 2)
      .endAngle(Math.PI / 2)

    g.append("path")
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      .attr("d", backgroundArc as any)
      .attr("fill", "#e5e7eb")

    // Foreground arc
    const foregroundArc = d3
      .arc()
      .innerRadius(radius - 7)
      .outerRadius(radius)
      .startAngle(-Math.PI / 2)
      .endAngle(-Math.PI / 2 + Math.PI * winRateValue)

    g.append("path")
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      .attr("d", foregroundArc as any)
      .attr("fill", "url(#gauge-gradient)")

    // Add tick marks
    const tickData = [0, 0.5, 1]
    tickData.forEach((tick) => {
      const angle = -Math.PI / 2 + Math.PI * tick
      const x1 = Math.cos(angle) * (radius - 10)
      const y1 = Math.sin(angle) * (radius - 10)
      const x2 = Math.cos(angle) * (radius + 2)
      const y2 = Math.sin(angle) * (radius + 2)

      g.append("line")
        .attr("x1", x1)
        .attr("y1", y1)
        .attr("x2", x2)
        .attr("y2", y2)
        .attr("stroke", "#9ca3af")
        .attr("stroke-width", 1)
    })

    // Add labels
    g.append("text")
      .attr("x", -radius - 4)
      .attr("y", 4)
      .attr("text-anchor", "end")
      .attr("font-size", "9px")
      .attr("fill", "#6b7280")
      .text("0")

    g.append("text")
      .attr("x", 0)
      .attr("y", -radius - 4)
      .attr("text-anchor", "middle")
      .attr("font-size", "9px")
      .attr("fill", "#6b7280")
      .text("50")

    g.append("text")
      .attr("x", radius + 4)
      .attr("y", 4)
      .attr("text-anchor", "start")
      .attr("font-size", "9px")
      .attr("fill", "#6b7280")
      .text("100")
  }, [winRate])

  const winRatePercent = winRate != null ? (winRate > 1 ? winRate : winRate * 100) : null

  return (
    <Card className="bg-white min-h-[138px]">
      <CardHeader className="pb-1">
        <div className="flex items-center gap-1.5">
          <CardTitle className="text-xs font-normal text-gray-600">Trade Win %</CardTitle>
          <Info className="w-3 h-3 text-gray-400" />
        </div>
      </CardHeader>
      <CardContent className="flex items-center justify-between pt-0 pb-1">
        <div className="text-2xl font-bold text-gray-900">{winRatePercent != null ? `${winRatePercent.toFixed(2)}%` : 'N/A'}</div>
        <svg ref={gaugeRef} width={90} height={36} />
      </CardContent>
    </Card>
  )
}

function AvgWinLossCard({
  averageWin,
  averageLoss,
}: {
  averageWin: number | null | undefined
  averageLoss: number | null | undefined
}) {
  const barRef = useRef<SVGSVGElement>(null)
  // Inline indicator dimensions
  const [dimensions] = useState({ width: 120, height: 36 })

  useEffect(() => {
    const width = dimensions.width
    if (!barRef.current || width === 0 || averageWin == null || averageLoss == null) return

    const svg = d3.select(barRef.current)
    svg.selectAll("*").remove()

    const barHeight = 14
    const centerY = 18
    const maxVal = Math.max(averageWin, Math.abs(averageLoss))
    const centerX = width / 2
    const maxBarWidth = width / 2 - 18

    // Win bar (right side, green)
    const winBarWidth = (averageWin / maxVal) * maxBarWidth
    svg
      .append("rect")
      .attr("x", centerX)
      .attr("y", centerY - barHeight / 2)
      .attr("width", winBarWidth)
      .attr("height", barHeight)
      .attr("fill", "#10b981")
      .attr("rx", 3)

    // Loss bar (left side, red)
    const lossBarWidth = (Math.abs(averageLoss) / maxVal) * maxBarWidth
    svg
      .append("rect")
      .attr("x", centerX - lossBarWidth)
      .attr("y", centerY - barHeight / 2)
      .attr("width", lossBarWidth)
      .attr("height", barHeight)
      .attr("fill", "#ef4444")
      .attr("rx", 3)

    // Labels
    svg
      .append("text")
      .attr("x", centerX + winBarWidth + 4)
      .attr("y", centerY + 4)
      .attr("fill", "#10b981")
      .attr("font-size", "9px")
      .attr("font-weight", "600")
      .text(`$${averageWin.toFixed(0)}`)

    svg
      .append("text")
      .attr("x", centerX - lossBarWidth - 4)
      .attr("y", centerY + 4)
      .attr("fill", "#ef4444")
      .attr("font-size", "9px")
      .attr("font-weight", "600")
      .attr("text-anchor", "end")
      .text(`-$${Math.abs(averageLoss).toFixed(0)}`)
  }, [averageWin, averageLoss, dimensions.width, dimensions.height])

  const ratio = (averageWin != null && averageLoss != null) ? averageWin / Math.abs(averageLoss) : null

  return (
    <Card className="bg-white min-h-[138px]">
      <CardHeader className="pb-1">
        <div className="flex items-center gap-1.5">
          <CardTitle className="text-xs font-normal text-gray-600">Avg win/loss trade</CardTitle>
          <Info className="w-3 h-3 text-gray-400" />
        </div>
      </CardHeader>
      <CardContent className="pt-0 pb-1">
        <div className="flex items-center gap-2">
          <div className="text-2xl font-bold text-gray-900">{ratio != null ? ratio.toFixed(2) : 'N/A'}</div>
          <svg ref={barRef} width={dimensions.width} height={dimensions.height} />
        </div>
      </CardContent>
    </Card>
  )
}
