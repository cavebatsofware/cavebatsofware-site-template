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

import React from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { useAuth } from "../contexts/AuthContext";
import "./Layout.css";

function Layout({ children }) {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();

  const isActive = (path) => {
    return location.pathname === path;
  };

  return (
    <div className="layout">
      <header className="layout-header">
        <div className="header-content">
          <div className="header-left">
            <h1 onClick={() => navigate("/dashboard")} className="site-title">
              Admin Dashboard
            </h1>
            <nav className="header-nav">
              <button
                className={`nav-link ${isActive("/dashboard") ? "active" : ""}`}
                onClick={() => navigate("/dashboard")}
              >
                Dashboard
              </button>
              <button
                className={`nav-link ${isActive("/access-codes") ? "active" : ""}`}
                onClick={() => navigate("/access-codes")}
              >
                Access Codes
              </button>
              <button
                className={`nav-link ${isActive("/access-logs") ? "active" : ""}`}
                onClick={() => navigate("/access-logs")}
              >
                Access Logs
              </button>
              <button
                className={`nav-link ${isActive("/admin-users") ? "active" : ""}`}
                onClick={() => navigate("/admin-users")}
              >
                Admin Users
              </button>
              <button
                className={`nav-link ${isActive("/settings") ? "active" : ""}`}
                onClick={() => navigate("/settings")}
              >
                Settings
              </button>
              <button
                className={`nav-link ${isActive("/profile") ? "active" : ""}`}
                onClick={() => navigate("/profile")}
              >
                Profile
              </button>
            </nav>
          </div>
          <div className="header-right">
            <span className="user-email">{user?.email}</span>
            <button onClick={logout} className="btn-logout">
              Logout
            </button>
          </div>
        </div>
      </header>

      <main className="layout-main">{children}</main>
    </div>
  );
}

export default Layout;
