import { GraphQLProvider } from "@/lib/client";
import { DashboardShell } from "@/components/dashboard-shell";

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <GraphQLProvider>
      <DashboardShell>{children}</DashboardShell>
    </GraphQLProvider>
  );
}
