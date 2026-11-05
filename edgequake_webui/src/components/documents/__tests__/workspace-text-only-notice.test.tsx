/**
 * @vitest-environment jsdom
 *
 * SPEC-185 — the "text only" note sits above the upload drop target and starts collapsed.
 *
 * What these tests defend:
 *  - the note never occupies the drop target's line (the measured regression: the settings
 *    selects went to 0px wide, so the upload controls disappeared behind it);
 *  - collapsed by default, because the condition is rare and the fact is permanent;
 *  - the reason and the fix are one click away, wired for screen readers;
 *  - dismissing it is remembered per workspace.
 */
import i18n from "@/lib/i18n";
import { resetTextOnlyNoticeDismissals } from "@/lib/documents/text-only-notice-dismiss";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { WorkspaceTextOnlyNotice } from "../workspace-text-only-notice";

const TEXT_ONLY = { embeds_images: false };
const IMAGES = { embeds_images: true };

function renderNotice(
  props: React.ComponentProps<typeof WorkspaceTextOnlyNotice> = {
    capability: TEXT_ONLY,
    workspaceId: "ws-1",
  },
) {
  return render(<WorkspaceTextOnlyNotice {...props} />);
}

describe("WorkspaceTextOnlyNotice", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    localStorage.clear();
    resetTextOnlyNoticeDismissals();
  });

  afterEach(() => {
    cleanup();
    localStorage.clear();
    resetTextOnlyNoticeDismissals();
  });

  it("states the limitation in one line and hides the detail until asked", () => {
    renderNotice();

    const note = screen.getByTestId("dropzone-text-only-notice");
    expect(note).toHaveTextContent("This workspace stores text only");
    // Four lines of prose above the primary action is what made the page feel clunky.
    expect(screen.queryByTestId("dropzone-text-only-notice-body")).not.toBeInTheDocument();
    expect(note).toHaveAttribute("data-expanded", "false");
  });

  it("opens the reason and the fix on click, and closes again", () => {
    renderNotice();
    const toggle = screen.getByTestId("dropzone-text-only-notice-toggle");
    expect(toggle).toHaveAttribute("aria-expanded", "false");

    fireEvent.click(toggle);

    const body = screen.getByTestId("dropzone-text-only-notice-body");
    expect(body).toHaveTextContent(/Text and images/i);
    expect(body).toHaveTextContent(/workspace/i);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    // The control points at the paragraph a screen reader should read next.
    expect(toggle).toHaveAttribute("aria-controls", body.id);

    fireEvent.click(toggle);
    expect(screen.queryByTestId("dropzone-text-only-notice-body")).not.toBeInTheDocument();
  });

  it("says nothing when the workspace can store pictures", () => {
    renderNotice({ capability: IMAGES, workspaceId: "ws-1" });

    expect(screen.queryByTestId("dropzone-text-only-notice")).not.toBeInTheDocument();
  });

  it("says nothing when the server has not stated the capability", () => {
    // Absent is unknown, not "no": guessing would put a refusal in front of a workspace
    // whose payload simply predates the field.
    renderNotice({ capability: null, workspaceId: "ws-1" });
    expect(screen.queryByTestId("dropzone-text-only-notice")).not.toBeInTheDocument();

    cleanup();
    renderNotice({ capability: undefined, workspaceId: "ws-1" });
    expect(screen.queryByTestId("dropzone-text-only-notice")).not.toBeInTheDocument();
  });

  it("remembers a dismissal for that workspace only", () => {
    renderNotice();
    fireEvent.click(screen.getByTestId("dropzone-text-only-notice-dismiss"));
    expect(screen.queryByTestId("dropzone-text-only-notice")).not.toBeInTheDocument();
    cleanup();

    renderNotice();
    expect(screen.queryByTestId("dropzone-text-only-notice")).not.toBeInTheDocument();
    cleanup();

    // A workspace the person has not seen still explains itself.
    renderNotice({ capability: TEXT_ONLY, workspaceId: "ws-other" });
    expect(screen.getByTestId("dropzone-text-only-notice")).toBeInTheDocument();
  });

  it("offers no dismiss control without a workspace to remember it under", () => {
    renderNotice({ capability: TEXT_ONLY, workspaceId: null });

    expect(screen.getByTestId("dropzone-text-only-notice")).toBeInTheDocument();
    expect(screen.queryByTestId("dropzone-text-only-notice-dismiss")).not.toBeInTheDocument();
  });
});
