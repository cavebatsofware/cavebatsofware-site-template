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

import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useAuth } from "../contexts/AuthContext";
import { fetchApi } from "../utils/api";
import PasswordChangeForm from "../components/PasswordChangeForm";
import "./ForcePasswordChange.css";

function ForcePasswordChange() {
  const { user, logout, refreshUser } = useAuth();
  const navigate = useNavigate();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function handleSubmit({ currentPassword, newPassword }) {
    setLoading(true);
    setError("");

    try {
      const response = await fetchApi("/api/admin/change-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          current_password: currentPassword,
          new_password: newPassword,
        }),
      });

      if (!response.ok) {
        const data = await response.json();
        throw new Error(data.error || "Failed to change password");
      }

      // Refresh user data to clear force_password_change flag
      if (refreshUser) {
        await refreshUser();
      }

      // Redirect to dashboard
      navigate("/dashboard");
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  async function handleLogout() {
    try {
      await logout();
      navigate("/login");
    } catch (err) {
      console.error("Logout failed:", err);
    }
  }

  return (
    <div className="force-password-change-page">
      <div className="force-password-change-container">
        <div className="force-password-change-card">
          <div className="force-password-change-header">
            <div className="warning-icon">!</div>
            <h1>Password Change Required</h1>
            <p>
              Your password was set by an administrator. For security reasons, you
              must choose a new password before continuing.
            </p>
          </div>

          {error && <div className="alert alert-error">{error}</div>}

          <PasswordChangeForm
            requireCurrentPassword={true}
            onSubmit={handleSubmit}
            loading={loading}
            email={user?.email}
          />

          <div className="force-password-change-footer">
            <button
              type="button"
              className="btn-link"
              onClick={handleLogout}
              disabled={loading}
            >
              Logout Instead
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default ForcePasswordChange;
