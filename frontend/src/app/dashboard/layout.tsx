import { GraphQLProvider } from "@/lib/client";

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <GraphQLProvider>{children}</GraphQLProvider>;
}
