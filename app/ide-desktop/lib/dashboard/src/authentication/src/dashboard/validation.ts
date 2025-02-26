/** @file Validation patterns for text inputs. */

/** Regex pattern for valid AWS Cognito passwords. */
export const PASSWORD_PATTERN =
    '(?=.*[0-9])(?=.*[A-Z])(?=.*[a-z])(?=.*[ ^$*.\\[\\]\\{\\}\\(\\)?"!@#%&\\/,><\':;\\|_~`=+\\-])' +
    '[0-9A-Za-z ^$*.\\[\\]\\{\\}\\(\\)?"!@#%&\\/,><\':;\\|_~`=+\\-]{6,256}'
/** Human readable explanation of password requirements. */
export const PASSWORD_TITLE =
    'Your password must include numbers, letters (both lowercase and uppercase) and symbols, ' +
    'and must be between 6 and 256 characters long.'

/** Regex pattern used by the backend for validating the previous password,
 * when changing password. */
export const PREVIOUS_PASSWORD_PATTERN = '^[\\S]+.*[\\S]+$'
/** Human readable explanation of password requirements. */
export const PREVIOUS_PASSWORD_TITLE =
    'Your password must neither start nor end with whitespace, and must contain ' +
    'at least two characters.'
