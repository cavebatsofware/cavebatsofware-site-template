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

import { useState, useEffect } from "react";
import { Link, useSearchParams, useNavigate } from "react-router-dom";
import { fetchApi } from "../utils/api";
import PasswordChangeForm from "../components/PasswordChangeForm";
import "./ResetPassword.css";

function ResetPassword() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState(false);
  const token = searchParams.get("token");

  useEffect(() => {
    if (!token) {
      setError("Invalid or missing reset token. Please request a new password reset.");
    }
  }, [token]);

  async function handleSubmit({ newPassword }) {
    setLoading(true);
    setError("");

    try {
      const response = await fetchApi("/api/admin/reset-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ token, new_password: newPassword }),
      });

      if (!response.ok) {
        const data = await response.json();
        throw new Error(data.error || "Failed to reset password");
      }

      setSuccess(true);
      // Redirect to login after a brief delay
      setTimeout(() => {
        navigate("/login");
      }, 3000);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="reset-password-page">
      <div className="reset-password-container">
        <div className="reset-password-card">
          <div className="reset-password-header">
            <h1>Set New Password</h1>
            {!success && (
              <p>Enter your new password below.</p>
            )}
          </div>

          {error && <div className="alert alert-error">{error}</div>}

          {success ? (
            <div className="success-message">
              <div className="success-icon">&#10003;</div>
              <p>Your password has been reset successfully!</p>
              <p className="success-note">
                Redirecting to login page...
              </p>
              <Link to="/login" className="btn-primary btn-full">
                Go to Login
              </Link>
            </div>
          ) : token ? (
            <PasswordChangeForm
              requireCurrentPassword={false}
              onSubmit={handleSubmit}
              loading={loading}
              email=""
            />
          ) : (
            <div className="invalid-token">
              <p>
                The reset link is invalid or has expired. Please request a new
                password reset.
              </p>
              <Link to="/forgot-password" className="btn-primary btn-full">
                Request New Reset
              </Link>
            </div>
          )}

          <div className="reset-password-footer">
            <Link to="/login">Back to Login</Link>
          </div>
        </div>
      </div>
    </div>
  );
}

export default ResetPassword;
