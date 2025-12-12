/*  This file is part of a basic website template project - cavebatsofware-site-template
 *  Copyright (C) 2025 Grant DeFayette & Cavebatsoftware LLC
 *
 *  cavebatsofware-site-template is free software: you can redistribute it and/or modify
 *  it under the terms of either the GNU General Public License as published by
 *  the Free Software Foundation, version 3 of the License (GPL-3.0-only), OR under
 *  the 3 clause BSD License (BSD-3-Clause).
 *  
 *  If you wish to use this software under the GPL-3.0-only license, remove
 *  references to BSD-3-Clause and copies of the BSD-3-Clause license from copies you distribute,
 *  unless you would like to dual-license your modifications to the software.
 *  
 *  If you wish to use this software under the BSD-3-Clause license, remove
 *  references to GPL-3.0-only and copies of the GPL-3.0-only License from copies you distribute,
 *  unless you would like to dual-license your modifications to the software.
 *
 *  cavebatsofware-site-template is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License and BSD 3 Clause License
 *  along with cavebatsofware-site-template.  If not, see <https://www.gnu.org/licenses/gpl-3.0.html>.
 *  For BSD-3-Clause terms, see <https://opensource.org/licenses/BSD-3-Clause>
 */

import React, { useState, useMemo } from "react";
import "./PasswordChangeForm.css";

const PASSWORD_REQUIREMENTS = {
  minLength: 16,
  maxLength: 128,
  hasUppercase: /[A-Z]/,
  hasLowercase: /[a-z]/,
  hasNumber: /[0-9]/,
  hasSpecial: /[!@#$%^&*()_+\-=\[\]{}\\|;':",.<>?\/`~]/,
};

function PasswordChangeForm({
  requireCurrentPassword = true,
  onSubmit,
  loading = false,
  email = "",
}) {
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");

  const validation = useMemo(() => {
    const checks = {
      length: newPassword.length >= PASSWORD_REQUIREMENTS.minLength,
      maxLength: newPassword.length <= PASSWORD_REQUIREMENTS.maxLength,
      uppercase: PASSWORD_REQUIREMENTS.hasUppercase.test(newPassword),
      lowercase: PASSWORD_REQUIREMENTS.hasLowercase.test(newPassword),
      number: PASSWORD_REQUIREMENTS.hasNumber.test(newPassword),
      special: PASSWORD_REQUIREMENTS.hasSpecial.test(newPassword),
      match: newPassword === confirmPassword && confirmPassword.length > 0,
      noEmail:
        !email ||
        !newPassword.toLowerCase().includes(email.toLowerCase().split("@")[0]),
    };

    const allValid =
      checks.length &&
      checks.maxLength &&
      checks.uppercase &&
      checks.lowercase &&
      checks.number &&
      checks.special &&
      checks.match &&
      checks.noEmail;

    return { checks, allValid };
  }, [newPassword, confirmPassword, email]);

  function handleSubmit(e) {
    e.preventDefault();
    if (!validation.allValid) return;
    if (requireCurrentPassword && !currentPassword) return;

    onSubmit({
      currentPassword: requireCurrentPassword ? currentPassword : undefined,
      newPassword,
    });
  }

  function resetForm() {
    setCurrentPassword("");
    setNewPassword("");
    setConfirmPassword("");
  }

  return (
    <form onSubmit={handleSubmit} className="password-change-form">
      {requireCurrentPassword && (
        <div className="form-group">
          <label htmlFor="current-password">Current Password</label>
          <input
            id="current-password"
            type="password"
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
            placeholder="Enter current password"
            autoComplete="current-password"
            disabled={loading}
          />
        </div>
      )}

      <div className="form-group">
        <label htmlFor="new-password">New Password</label>
        <input
          id="new-password"
          type="password"
          value={newPassword}
          onChange={(e) => setNewPassword(e.target.value)}
          placeholder="Enter new password"
          autoComplete="new-password"
          disabled={loading}
        />
      </div>

      <div className="form-group">
        <label htmlFor="confirm-password">Confirm New Password</label>
        <input
          id="confirm-password"
          type="password"
          value={confirmPassword}
          onChange={(e) => setConfirmPassword(e.target.value)}
          placeholder="Confirm new password"
          autoComplete="new-password"
          disabled={loading}
        />
      </div>

      <div className="password-requirements">
        <h4>Password Requirements</h4>
        <ul className="requirements-list">
          <li className={validation.checks.length ? "valid" : "invalid"}>
            {validation.checks.length ? "✓" : "✗"} At least 16 characters (
            {newPassword.length}/16)
          </li>
          <li className={validation.checks.uppercase ? "valid" : "invalid"}>
            {validation.checks.uppercase ? "✓" : "✗"} At least one uppercase
            letter (A-Z)
          </li>
          <li className={validation.checks.lowercase ? "valid" : "invalid"}>
            {validation.checks.lowercase ? "✓" : "✗"} At least one lowercase
            letter (a-z)
          </li>
          <li className={validation.checks.number ? "valid" : "invalid"}>
            {validation.checks.number ? "✓" : "✗"} At least one number (0-9)
          </li>
          <li className={validation.checks.special ? "valid" : "invalid"}>
            {validation.checks.special ? "✓" : "✗"} At least one special
            character (!@#$%^&*...)
          </li>
          {email && (
            <li className={validation.checks.noEmail ? "valid" : "invalid"}>
              {validation.checks.noEmail ? "✓" : "✗"} Cannot contain your
              username
            </li>
          )}
          <li className={validation.checks.match ? "valid" : "invalid"}>
            {validation.checks.match ? "✓" : "✗"} Passwords match
          </li>
        </ul>
      </div>

      <div className="form-actions">
        <button
          type="submit"
          className="btn-primary"
          disabled={
            loading ||
            !validation.allValid ||
            (requireCurrentPassword && !currentPassword)
          }
        >
          {loading ? "Changing..." : "Change Password"}
        </button>
        <button
          type="button"
          className="btn-secondary"
          onClick={resetForm}
          disabled={loading}
        >
          Clear
        </button>
      </div>
    </form>
  );
}

export default PasswordChangeForm;
