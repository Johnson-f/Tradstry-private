import { DashboardCalendar, DashboardUpperCard } from "@/components/dashboard";

export default function Page() {
  return (
    <div className="space-y-6">
      <DashboardUpperCard />
      <DashboardCalendar />
    </div>
  );
}
