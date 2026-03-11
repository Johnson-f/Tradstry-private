<!-- 9d977309-b8f0-4cb6-971a-a616be2b5d6f 96c71bcc-8bd4-4088-bc1f-dad1317f808b -->
# D3.js Cumulative P&L Chart Implementation

## Overview

Build a minimal, clean cumulative P&L chart in `components/journal/basic-analytics.tsx` using D3.js that exactly matches the provided image design.

## Current State

- ✅ Backend provides time series data: `GET /api/analytics/time-series?time_range={7d|30d|90d|1y|ytd|all_time}`
- ✅ Returns `TimeSeriesPoint[]` with: `date`, `value`, `cumulative_value`, `trade_count`
- ✅ Frontend has `useAnalyticsTimeSeries()` hook for fetching data
- 📁 File to build: `components/journal/basic-analytics.tsx`

## D3.js Implementation Plan

### Component Structure

```typescript
// components/journal/basic-analytics.tsx
"use client"
import { useEffect, useRef, useState } from 'react'
import * as d3 from 'd3'
import { useAnalyticsTimeSeries } from '@/lib/hooks/use-analytics'

export default function BasicAnalytics() {
  const [timeRange, setTimeRange] = useState<'7d' | '30d' | '90d' | '1y' | 'ytd' | 'all_time'>('30d')
  const { data, isLoading, error } = useAnalyticsTimeSeries({ time_range: timeRange })
  const chartRef = useRef<HTMLDivElement>(null)
  const svgRef = useRef<SVGSVGElement>(null)

  useEffect(() => {
    if (!data || !data.daily_pnl || daily_pnl.length === 0) return
    
    // D3 chart rendering logic here
  }, [data])

  return (
    <div className="bg-white rounded-lg p-4">
      {/* Chart will be rendered here */}
    </div>
  )
}
```

### D3.js Chart Specifications

**Dimensions:**

- Card: `bg-white rounded-lg p-4` (or similar padding)
- SVG: Full width, responsive height (~300-400px)

**Visual Elements (to match image):**

1. **Title & Data** (text elements):

   - "Net Cumulative P&L" (smaller, grey font, top-left)
   - Data point count (e.g., "35") next to title
   - Large dollar amount (current cumulative_value) - bold, prominent

2. **Chart Area** (D3 SVG):

   - Line: smooth line connecting cumulative P&L points
   - Area: gradient-filled area beneath the line
   - No axes, grid lines, or labels (clean design)
   - Gradient colors: teal (#0f766e) to light green (#d1fae5) based on image

### Implementation Steps

**Step 1: Setup SVG Container**

- Create SVG element with width/height
- Set up margins for padding
- Define scales for x (time) and y (cumulative P&L)

**Step 2: Create Gradient**

- Define linearGradient with id "pnl-gradient"
- Add gradient stops: darker teal at bottom, lighter green at top
- Match gradient colors from image

**Step 3: Draw Area Path**

- Use `d3.area()` generator
- Map dates to x, cumulative_value to y
- Fill with gradient
- Baseline at y=0 or minimum value

**Step 4: Draw Line**

- Use `d3.line()` generator  
- Overlay on top of area
- Set stroke color and width

**Step 5: Add Data Points (optional)**

- Small dots/circles at each data point
- Visible on hover

**Step 6: Hover Interaction**

- Show tooltip with: date, daily P&L, cumulative P&L, trade count
- Highlight hovered point

**Step 7: Update on Time Range Change**

- Re-render when timeRange changes
- Clear previous chart before rendering new one

### Code Structure

```typescript
// Main component structure
<div className="bg-white rounded-lg shadow-sm p-4">
  {/* Header */}
  <div className="flex justify-between items-start mb-4">
    <div className="flex items-center gap-2">
      <span className="text-sm text-gray-600">Net Cumulative P&L</span>
      <span className="text-sm font-medium text-gray-700">{dataPointCount}</span>
    </div>
    <div className="text-4xl font-bold">{currentPnl}</div>
  </div>

  {/* D3 Chart */}
  <div ref={chartRef}>
    <svg ref={svgRef}></svg>
  </div>

  {/* Time Range Filter */}
  <div className="flex gap-2 mt-4">
    {/* Radio buttons or dropdown */}
  </div>
</div>
```

### D3.js Code Pattern

```typescript
// Inside useEffect
const svg = d3.select(svgRef.current)
const data = daily_pnl

// Setup scales
const xScale = d3.scaleTime()
  .domain(d3.extent(data, d => new Date(d.date)))
  .range([margin.left, width - margin.right])

const yScale = d3.scaleLinear()
  .domain([minCumulative, maxCumulative])
  .range([height - margin.bottom, margin.top])

// Define gradient
const gradient = svg.append("defs").append("linearGradient")
  .attr("id", "pnl-gradient")
  .attr("x1", "0%").attr("y1", "0%")
  .attr("x2", "0%").attr("y2", "100%")
gradient.append("stop").attr("offset", "0%").attr("stop-color", "#d1fae5")
gradient.append("stop").attr("offset", "100%").attr("stop-color", "#0f766e")

// Create area
const area = d3.area()
  .x(d => xScale(new Date(d.date)))
  .y0(yScale(0))
  .y1(d => yScale(d.cumulative_value))
  .curve(d3.curveMonotoneX)

// Create line
const line = d3.line()
  .x(d => xScale(new Date(d.date)))
  .y(d => yScale(d.cumulative_value))
  .curve(d3.curveMonotoneX)

// Draw paths
// Add hover interactions
```

### Tasks

1. Install d3 dependencies (`@types/d3`)
2. Create basic component structure with time range selector
3. Implement D3 SVG setup with scales
4. Add gradient definition (teal to light green)
5. Render area path with gradient fill
6. Render line path on top of area
7. Add hover tooltips with trade details
8. Handle time range changes and re-render
9. Add responsive behavior
10. Style to match image exactly

### Dependencies Needed - use bun to install the package

- `d3` package
- `@types/d3` for TypeScript

### To-dos

- [ ] Create cumulative P&L chart component using Recharts with line and markers
- [ ] Create useCumulativePnl hook to fetch and transform time series data
- [ ] Build basic-analytics.tsx UI with chart and time range filters
- [ ] Fetch trades data and add visual markers on chart for entry/exit
- [ ] Implement detailed hover tooltips showing date, daily P&L, cumulative P&L, and trade count