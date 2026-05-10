import { expect, test } from "@playwright/test";

test("app launches and creates a second workspace tab", async ({ page }) => {
  await page.goto("/");

  await expect(
    page.locator('[role="tab"]').filter({ hasText: "Tab 1" }),
  ).toBeVisible();
  await expect(page.getByTestId("input-editor")).toBeVisible();
  await expect(page.getByTestId("accepted-output-editor")).toBeVisible();
  await expect(page.getByTestId("generate-button")).toHaveText("Generate");
  await expect(page.getByTestId("generate-button")).toBeDisabled();
  await expect(page.getByTestId("status-bar")).toContainText("No model");

  await page.getByLabel("New tab").click();

  await expect(page.locator('[role="tab"]')).toHaveCount(2);
  await expect(
    page.locator('[role="tab"]').filter({ hasText: "Tab 2" }),
  ).toHaveAttribute("aria-selected", "true");

  await page
    .getByTestId("input-editor")
    .fill("Rewrite this in a concise tone.");
  await expect(page.getByTestId("input-editor")).toHaveValue(
    "Rewrite this in a concise tone.",
  );
});
