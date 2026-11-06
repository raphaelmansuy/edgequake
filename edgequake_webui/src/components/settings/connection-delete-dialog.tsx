"use client";

/**
 * SPEC-179 — delete a saved connection. The first attempt carries no confirm flag; when
 * assignments still use it the server answers 409 `confirm_required` and the dialog says how many.
 */

import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { ApiRequestError, apiClient } from "@/lib/api/client";
import { needsConfirm } from "@/lib/api/edgequake/provider-offer";

export interface DeletableConnection {
  id: string;
  display_name: string;
}

export function ConnectionDeleteDialog({
  connection,
  onClose,
  onDeleted,
}: {
  connection: DeletableConnection | null;
  onClose: () => void;
  onDeleted: () => void | Promise<unknown>;
}) {
  const { t } = useTranslation();
  const [inUse, setInUse] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const close = () => {
    setInUse(null);
    onClose();
  };

  const remove = async () => {
    if (!connection) return;
    setBusy(true);
    try {
      const confirm = inUse !== null ? "&confirm=true" : "";
      await apiClient(`/connections/${connection.id}?scope=deployment${confirm}`, { method: "DELETE", omitTenant: true });
      await onDeleted();
      close();
    } catch (error) {
      if (needsConfirm(error)) setInUse(error.message);
      else if (error instanceof ApiRequestError && error.code === "deployment_admin_required") {
        toast.error(
          error.message && error.message !== "Forbidden"
            ? error.message
            : t("settings.connections.deploymentAdmin", "Only a platform admin can add a provider on the deployment."),
        );
      } else toast.error(error instanceof Error ? error.message : t("settings.connections.deleteFailed", "Delete failed"));
    } finally {
      setBusy(false);
    }
  };

  return (
    <AlertDialog open={connection !== null} onOpenChange={(open) => !open && close()}>
      <AlertDialogContent data-testid="connection-delete-dialog">
        <AlertDialogHeader>
          <AlertDialogTitle>
            {t("settings.connections.deleteTitle", "Delete {{name}}?", { name: connection?.display_name ?? "" })}
          </AlertDialogTitle>
          <AlertDialogDescription data-testid="connection-delete-message">
            {inUse !== null
              ? `${inUse} ${t(
                  "settings.connections.deleteInUse",
                  "They fail with “not eligible” until an admin picks another model.",
                )}`
              : t("settings.connections.deleteBody", "Its stored key is removed with it. This cannot be undone.")}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={busy}>{t("common.cancel", "Cancel")}</AlertDialogCancel>
          <Button
            type="button"
            variant="destructive"
            disabled={busy}
            data-testid="connection-delete-confirm"
            onClick={() => void remove()}
          >
            {inUse !== null
              ? t("settings.connections.deleteAnyway", "Delete anyway")
              : t("settings.connections.delete", "Delete")}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
