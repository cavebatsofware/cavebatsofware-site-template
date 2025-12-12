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

import React, { useState, useEffect } from "react";
import Layout from "../components/Layout";
import Table from "../components/Table";
import { fetchApi } from "../utils/api";
import "./AccessLogs.css";

function AccessLogs() {
  const [logs, setLogs] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [currentPage, setCurrentPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [total, setTotal] = useState(0);
  const [perPage, setPerPage] = useState(100);

  useEffect(() => {
    fetchLogs(currentPage);
  }, [currentPage]);

  async function fetchLogs(page = 1) {
    try {
      setLoading(true);
      const response = await fetchApi(
        `/api/admin/access-logs?page=${page}&per_page=${perPage}`
      );

      if (!response.ok) {
        throw new Error("Failed to fetch access logs");
      }

      const data = await response.json();
      setLogs(data.data);
      setTotal(data.total);
      setTotalPages(data.total_pages);
      setCurrentPage(data.page);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  async function handleClearLogs() {
    if (
      !confirm(
        "Are you sure you want to clear all access logs? This action cannot be undone.",
      )
    ) {
      return;
    }

    try {
      const response = await fetchApi("/api/admin/access-logs", {
        method: "DELETE",
      });

      if (!response.ok) {
        throw new Error("Failed to clear access logs");
      }

      // Reset to first page
      setCurrentPage(1);
      await fetchLogs(1);
    } catch (err) {
      setError(err.message);
    }
  }

  function handlePageChange(newPage) {
    if (newPage >= 1 && newPage <= totalPages) {
      setCurrentPage(newPage);
    }
  }

  function formatDate(dateString) {
    if (!dateString) return "N/A";
    const date = new Date(dateString);
    return date.toLocaleDateString() + " " + date.toLocaleTimeString();
  }

  function truncateUserAgent(userAgent) {
    if (!userAgent) return "N/A";
    return userAgent.length > 50
      ? userAgent.substring(0, 50) + "..."
      : userAgent;
  }

  const columns = [
    {
      key: "created_at",
      header: "Timestamp",
      render: (value) => formatDate(value),
    },
    {
      key: "access_code",
      header: "Path",
      render: (value) => <code>{value}</code>,
    },
    {
      key: "ip_address",
      header: "IP Address",
      render: (value) => value || "N/A",
    },
    {
      key: "action",
      header: "Action",
      render: (value) => (
        <span className={`badge badge-method badge-${value.toLowerCase()}`}>
          {value}
        </span>
      ),
    },
    {
      key: "admin_user_email",
      header: "Admin User",
      render: (value) =>
        value ? (
          <span className="badge badge-admin" title={value}>
            {value.split("@")[0]}
          </span>
        ) : (
          <span className="text-muted">-</span>
        ),
    },
    {
      key: "success",
      header: "Status",
      render: (value) => (
        <span className={`badge ${value ? "badge-success" : "badge-failed"}`}>
          {value ? "Success" : "Failed"}
        </span>
      ),
    },
    {
      key: "user_agent",
      header: "User Agent",
      render: (value, row) => (
        <span title={row.user_agent}>{truncateUserAgent(value)}</span>
      ),
    },
    {
      key: "tokens",
      header: "Tokens",
      render: (value) => value || 0,
    },
  ];

  return (
    <Layout>
      <div className="access-logs-page">
        <header className="page-header">
          <h1>Access Logs</h1>
          <button onClick={handleClearLogs} className="btn-danger">
            Clear All Logs
          </button>
        </header>

        {error && <div className="error">{error}</div>}

        <div className="logs-stats">
          <div className="stat-card">
            <div className="stat-label">Total Logs</div>
            <div className="stat-value">{total}</div>
          </div>
          <div className="stat-card">
            <div className="stat-label">Successful Accesses</div>
            <div className="stat-value">
              {logs.filter((log) => log.success).length}
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-label">Failed Attempts</div>
            <div className="stat-value">
              {logs.filter((log) => !log.success).length}
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-label">Current Page</div>
            <div className="stat-value">
              {currentPage} / {totalPages}
            </div>
          </div>
        </div>

        <Table
          columns={columns}
          data={logs}
          loading={loading}
          emptyMessage="No access logs yet."
          getRowClassName={(row) => (row.success ? "" : "failed")}
          pagination={{
            page: currentPage,
            totalPages,
            onPageChange: handlePageChange,
          }}
        />
      </div>
    </Layout>
  );
}

export default AccessLogs;
