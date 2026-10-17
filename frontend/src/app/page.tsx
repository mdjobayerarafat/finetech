"use client";

import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { useAuth } from "@/lib/store";

export default function Home() {
  const router = useRouter();
  const { me, loading } = useAuth();
  useEffect(() => {
    if (loading) return;
    if (!me) {
      router.replace("/login");
    } else {
      router.replace(me.role === "admin" ? "/admin" : "/dashboard");
    }
  }, [me, loading, router]);
  return (
    <div className="grid min-h-dvh place-items-center text-sm text-muted-foreground">
      Loading…
    </div>
  );
}
