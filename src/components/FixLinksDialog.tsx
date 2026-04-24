import { useState } from "react";
import { X, Wrench, Loader2, Link, Trash2, Download } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { cn } from "../utils";
import { getErrorMessage } from "../lib/error";
import type { SkillLinkIssue } from "../lib/tauri";
import * as api from "../lib/tauri";

interface Props {
  open: boolean;
  issues: SkillLinkIssue[];
  onClose: () => void;
  onRefresh: () => Promise<void>;
}

export function FixLinksDialog({ open, issues, onClose, onRefresh }: Props) {
  const { t } = useTranslation();
  const [fixingAll, setFixingAll] = useState(false);
  const [fixingItem, setFixingItem] = useState<string | null>(null);
  const [localIssues, setLocalIssues] = useState<SkillLinkIssue[]>(issues);

  // Sync when issues prop changes (dialog re-opened)
  if (issues !== localIssues && issues.length > 0 && localIssues.length === 0) {
    setLocalIssues(issues);
  }

  if (!open) return null;

  const itemKey = (issue: SkillLinkIssue) =>
    `${issue.tool_key}:${issue.skill_name}`;

  const handleFixAll = async () => {
    setFixingAll(true);
    try {
      const result = await api.fixAllSkillLinks();
      toast.success(
        t("mySkills.fixLinks.fixAllDone", {
          relinked: result.relinked,
          deleted: result.deleted,
          skipped: result.skipped,
        })
      );
      await onRefresh();
      // Re-diagnose to update the list
      const remaining = await api.diagnoseSkillLinks();
      setLocalIssues(remaining);
      if (remaining.length === 0) {
        onClose();
      }
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, "Fix failed"));
    } finally {
      setFixingAll(false);
    }
  };

  const handleFixItem = async (
    issue: SkillLinkIssue,
    action: string,
    successKey: string
  ) => {
    const key = itemKey(issue);
    setFixingItem(key);
    try {
      await api.fixSkillLink(issue.tool_key, issue.skill_name, action);
      toast.success(
        t(`mySkills.fixLinks.${successKey}`, {
          skill: issue.skill_name,
          agent: issue.tool_display_name,
        })
      );
      await onRefresh();
      setLocalIssues((prev) =>
        prev.filter((i) => itemKey(i) !== key)
      );
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, "Fix failed"));
    } finally {
      setFixingItem(null);
    }
  };

  const issueTypeLabel = (type: string) =>
    t(`mySkills.fixLinks.issueType.${type}`, type);

  const issueColor = (type: string) => {
    switch (type) {
      case "broken_symlink":
        return "text-red-500 dark:text-red-400";
      case "physical_copy_identical":
        return "text-amber-600 dark:text-amber-400";
      case "physical_copy_diverged":
        return "text-orange-600 dark:text-orange-400";
      case "orphan":
        return "text-violet-600 dark:text-violet-400";
      case "wrong_target_symlink":
        return "text-rose-600 dark:text-rose-400";
      default:
        return "text-muted";
    }
  };

  const safeFixableCount = localIssues.filter(
    (i) =>
      i.issue_type === "physical_copy_identical" ||
      i.issue_type === "broken_symlink"
  ).length;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div
        className="absolute inset-0 bg-black/70 backdrop-blur-sm"
        onClick={onClose}
      />
      <div className="relative flex max-h-[80vh] w-full max-w-2xl flex-col rounded-xl border border-border bg-surface shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border-subtle px-5 py-4">
          <div>
            <h2 className="flex items-center gap-2 text-[14px] font-semibold text-primary">
              <Wrench className="h-4 w-4 text-accent" />
              {t("mySkills.fixLinks.title")}
            </h2>
            <p className="mt-0.5 text-[12px] text-muted">
              {t("mySkills.fixLinks.description", {
                count: localIssues.length,
              })}
            </p>
          </div>
          <div className="flex items-center gap-2">
            {safeFixableCount > 0 && (
              <button
                onClick={handleFixAll}
                disabled={fixingAll || !!fixingItem}
                className="inline-flex items-center gap-1.5 rounded-md bg-accent-dark px-3 py-1.5 text-[13px] font-medium text-white transition-colors hover:bg-accent disabled:opacity-50"
              >
                {fixingAll ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Wrench className="h-3.5 w-3.5" />
                )}
                {t("mySkills.fixLinks.fixAllSafe")}
              </button>
            )}
            <button
              onClick={onClose}
              className="rounded p-1 text-muted transition-colors hover:text-secondary"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </div>

        {/* Issue list */}
        <div className="flex-1 overflow-auto px-5 py-3">
          {localIssues.length === 0 ? (
            <p className="py-8 text-center text-[13px] text-muted">
              {t("mySkills.fixLinks.healthy")}
            </p>
          ) : (
            <table className="w-full text-[13px]">
              <thead>
                <tr className="border-b border-border-subtle text-left text-[12px] font-medium text-faint">
                  <th className="pb-2 pr-3">
                    {t("mySkills.fixLinks.columnAgent")}
                  </th>
                  <th className="pb-2 pr-3">
                    {t("mySkills.fixLinks.columnSkill")}
                  </th>
                  <th className="pb-2 pr-3">
                    {t("mySkills.fixLinks.columnIssue")}
                  </th>
                  <th className="pb-2 text-right">
                    {t("mySkills.fixLinks.columnAction")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {localIssues.map((issue) => {
                  const key = itemKey(issue);
                  const isFixing = fixingItem === key;
                  return (
                    <tr
                      key={key}
                      className="border-b border-border-subtle/50 last:border-0"
                    >
                      <td className="py-2.5 pr-3 text-secondary">
                        {issue.tool_display_name}
                      </td>
                      <td className="py-2.5 pr-3 font-medium text-primary">
                        {issue.skill_name}
                      </td>
                      <td
                        className={cn(
                          "py-2.5 pr-3 font-medium",
                          issueColor(issue.issue_type)
                        )}
                      >
                        {issueTypeLabel(issue.issue_type)}
                      </td>
                      <td className="py-2.5 text-right">
                        <div className="inline-flex items-center gap-1">
                          {/* Relink — available when central exists */}
                          {issue.central_path && (
                            <button
                              onClick={() =>
                                handleFixItem(issue, "relink", "relinked")
                              }
                              disabled={isFixing || fixingAll}
                              className="inline-flex items-center gap-1 rounded px-2 py-1 text-[12px] font-medium text-accent-light transition-colors hover:bg-accent-bg disabled:opacity-50"
                              title={t("mySkills.fixLinks.actionRelink")}
                            >
                              {isFixing ? (
                                <Loader2 className="h-3 w-3 animate-spin" />
                              ) : (
                                <Link className="h-3 w-3" />
                              )}
                              {t("mySkills.fixLinks.actionRelink")}
                            </button>
                          )}
                          {/* Import & Relink — for orphans and diverged without central */}
                          {(issue.issue_type === "orphan" ||
                            issue.issue_type === "physical_copy_diverged") &&
                            !issue.central_path && (
                              <button
                                onClick={() =>
                                  handleFixItem(
                                    issue,
                                    "import_and_relink",
                                    "imported"
                                  )
                                }
                                disabled={isFixing || fixingAll}
                                className="inline-flex items-center gap-1 rounded px-2 py-1 text-[12px] font-medium text-emerald-600 transition-colors hover:bg-emerald-500/10 disabled:opacity-50 dark:text-emerald-400"
                                title={t("mySkills.fixLinks.actionImport")}
                              >
                                {isFixing ? (
                                  <Loader2 className="h-3 w-3 animate-spin" />
                                ) : (
                                  <Download className="h-3 w-3" />
                                )}
                                {t("mySkills.fixLinks.actionImport")}
                              </button>
                            )}
                          {/* Delete — always available */}
                          <button
                            onClick={() =>
                              handleFixItem(issue, "delete", "deleted")
                            }
                            disabled={isFixing || fixingAll}
                            className="inline-flex items-center gap-1 rounded px-2 py-1 text-[12px] font-medium text-faint transition-colors hover:text-red-400 disabled:opacity-50"
                            title={t("mySkills.fixLinks.actionDelete")}
                          >
                            {isFixing ? (
                              <Loader2 className="h-3 w-3 animate-spin" />
                            ) : (
                              <Trash2 className="h-3 w-3" />
                            )}
                            {t("mySkills.fixLinks.actionDelete")}
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-end border-t border-border-subtle px-5 py-3">
          <button
            onClick={onClose}
            className="rounded-md px-3 py-1.5 text-[13px] font-medium text-tertiary transition-colors hover:bg-surface-hover hover:text-secondary"
          >
            {t("mySkills.fixLinks.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
