"use client";

import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { AdminShell } from "@/components/admin/admin-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { auditApi, type SafetyEvent } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { useToast } from "@/components/ui/toast";
import { formatDateTime } from "@/lib/utils";
import { ShieldAlert, Unlock, RefreshCw } from "lucide-react";

export default function AdminSafetyPage() {
  return (
    <AdminShell>
      <Body />
    </AdminShell>
  );
}

function Body() {
  const [events, setEvents] = useState<SafetyEvent[]>([]);
  const { toast } = useToast();

  const refresh = () => auditApi.safetyEvents().then(setEvents).catch(() => {});

  useEffect(() => {
    refresh();
  }, []);

  const unlock = async (user_id: string) => {
    try {
      await auditApi.unlock(user_id, "Admin unlock from console");
      toast({ variant: "success", title: "User unlocked" });
      refresh();
    } catch {
      toast({ variant: "error", title: "Unlock failed" });
    }
  };

  // Group by user
  const byUser: Record<string, SafetyEvent[]> = {};
  for (const ev of events) {
    (byUser[ev.user_id] ??= []).push(ev);
  }

  return (
    <div className="space-y-6">
      <FadeIn>
        <div className="flex items-end justify-between gap-3">
          <div>
            <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
              Admin · safety
            </p>
            <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">
              Safety events
            </h1>
            <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
              Failed MFAs, denied high-risk actions, rapid high-value
              attempts, and admin unlocks land here.
            </p>
          </div>
          <Button variant="outline" onClick={refresh}>
            <RefreshCw className="size-4" />
            Refresh
          </Button>
        </div>
      </FadeIn>

      {Object.keys(byUser).length === 0 ? (
        <FadeIn delay={0.1}>
          <Card>
            <CardContent className="grid h-32 place-items-center text-sm text-muted-foreground">
              No safety events recorded. Stay vigilant.
            </CardContent>
          </Card>
        </FadeIn>
      ) : (
        <Stagger>
          <div className="space-y-4">
            {Object.entries(byUser).map(([userId, evs]) => (
              <StaggerItem key={userId}>
                <Card>
                  <CardHeader>
                    <div className="flex flex-wrap items-end justify-between gap-2">
                      <div>
                        <CardTitle>User {userId.slice(0, 16)}…</CardTitle>
                        <CardDescription>{evs.length} event(s)</CardDescription>
                      </div>
                      <Button size="sm" variant="outline" onClick={() => unlock(userId)}>
                        <Unlock className="size-3.5" />
                        Unlock
                      </Button>
                    </div>
                  </CardHeader>
                  <CardContent>
                    <ul className="space-y-2">
                      {evs.map((ev) => (
                        <motion.li
                          layout
                          key={ev.id}
                          className="flex items-start justify-between gap-3 rounded-md border bg-background/30 p-3 text-sm"
                        >
                          <div>
                            <div className="flex items-center gap-2">
                              <ShieldAlert className="size-3.5 text-tier-yellow" />
                              <code className="font-mono text-xs">{ev.kind}</code>
                            </div>
                            <pre className="mt-1 text-[10px] text-muted-foreground">
                              {JSON.stringify(ev.metadata, null, 2)}
                            </pre>
                          </div>
                          <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                            {formatDateTime(ev.created_at)}
                          </div>
                        </motion.li>
                      ))}
                    </ul>
                  </CardContent>
                </Card>
              </StaggerItem>
            ))}
          </div>
        </Stagger>
      )}
    </div>
  );
}
