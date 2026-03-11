import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { YouTubeEmbed } from "@/components/education"

type EducationVideo = {
  title: string
  videoId: string
}

const educationVideos: EducationVideo[] = [
  {
    title: "How to build a Profitable system with Marios",
    videoId: "iGEVChlsMfQ",
  },
  {
    title: "How Minervini became a market wizard",
    videoId: "uh5bALsKkLg",
  },
  {
    title: "Sell rules with Ameet",
    videoId: "ai43Gb3YSFI",
  },
  {
    title: "Stan Weistein talks",
    videoId: "96462EsmhU8",
  },
  {
    title: "Stages of trading - watch to know which stage you are in",
    videoId: "3v9sZciniCY",
  },
  {
    title: "Spotting IPO winners",
    videoId: "qCW5OIRAjXI",
  },
  {
    title: "High Tight flag setups - train your eyes",
    videoId: "WtHpJcQ6bHg",
  },
]

export function VideosList() {
  return (
    <div className="grid gap-6 md:grid-cols-2 xl:grid-cols-3">
      {educationVideos.map((video) => (
        <Card key={video.videoId} className="overflow-hidden">
          <CardHeader className="pb-3">
            <CardTitle className="text-base">{video.title}</CardTitle>
          </CardHeader>
          <CardContent>
            <YouTubeEmbed videoId={video.videoId} title={video.title} />
          </CardContent>
        </Card>
      ))}
    </div>
  )
}

