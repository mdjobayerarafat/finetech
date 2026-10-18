import { PageSkeleton } from "@/components/ui/page-skeleton";

export default function Loading() {
  return <PageSkeleton cards={3} gridCols="sm:grid-cols-2 lg:grid-cols-3" />;
}
