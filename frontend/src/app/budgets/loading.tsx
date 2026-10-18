import { PageSkeleton } from "@/components/ui/page-skeleton";

export default function Loading() {
  return <PageSkeleton kpis cards={3} gridCols="md:grid-cols-2 lg:grid-cols-3" />;
}
