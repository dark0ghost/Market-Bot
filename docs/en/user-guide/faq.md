# FAQ

## Tinkoff - "Token not found"
- Verify the token starts with `t.`
- Check that `API_TOKEN` env var is set or the token is in `account.json`
- Ensure the token has the required permissions in sandbox mode

## Tinkoff - Sandbox account not found
- Enable `"open_account": true` in the `sandbox` config section to auto-create on startup
- Check logs - the bot logs the new `account_id` after creation
- Sandbox accounts expire after 3 months of inactivity; create a new one if needed
- If you hardcode an `account_id` that no longer exists, the API will return errors - remove it from config to trigger auto-creation

## Finam - Authentication fails
- Verify your API secret is correct
- Contact Finam support to ensure API access is enabled on your account

## Mock - Order not filled
- Mock broker fills orders instantly. Check that:
  - The instrument price is set via `set_price()`
  - Available balance is sufficient
  - Position limits are respected
