import { PageSkeleton } from "@/components/ui/page-skeleton";

export default function Loading() {
  return <PageSkeleton cards={6} gridCols="md:grid-cols-3" />;
}
