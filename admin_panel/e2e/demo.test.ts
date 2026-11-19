import { expect, test } from '@playwright/test';

test('shows the login screen before data is loaded', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByText('Authentication Required')).toBeVisible();
	await expect(page.getByPlaceholder('Enter admin password')).toBeVisible();
});
