interface YouTubeEmbedProps {
  videoId: string; // e.g. "dQw4w9WgXcQ"
  title?: string;
  startSeconds?: number;
  className?: string;
}

export function YouTubeEmbed({
  videoId,
  title = "YouTube video",
  startSeconds,
  className,
}: YouTubeEmbedProps) {
  const params = new URLSearchParams({
    rel: "0",
    modestbranding: "1",
    playsinline: "1",
    ...(startSeconds ? { start: String(startSeconds) } : {}),
  });

  return (
    <div
      className={`relative w-full overflow-hidden rounded-lg bg-black ${className ?? ""}`}
      style={{ paddingBottom: "56.25%" }}
    >
      <iframe
        src={`https://www.youtube.com/embed/${videoId}?${params.toString()}`}
        title={title}
        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
        allowFullScreen
        loading="lazy"
        referrerPolicy="strict-origin-when-cross-origin"
        className="absolute inset-0 h-full w-full border-0"
      />
    </div>
  );
}

