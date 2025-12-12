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
import "./Table.css";

/**
 * Reusable Table Component
 *
 * @param {Array} columns - Column definitions [{ key, header, render? }]
 * @param {Array} data - Array of row data objects
 * @param {Function} getRowKey - Function to extract unique key from row (default: row => row.id)
 * @param {Function} getRowClassName - Optional function to add classes to rows (row) => string
 * @param {boolean} loading - Show loading state
 * @param {string} emptyMessage - Message when no data
 * @param {Object} pagination - Pagination config { page, totalPages, onPageChange }
 */
function Table({
  columns,
  data,
  getRowKey = (row) => row.id,
  getRowClassName,
  loading = false,
  emptyMessage = "No data available",
  pagination,
}) {
  if (loading) {
    return <div className="loading">Loading...</div>;
  }

  if (!data || data.length === 0) {
    return (
      <div className="empty-state">
        <p>{emptyMessage}</p>
      </div>
    );
  }

  return (
    <div className="table-container">
      <table className="data-table">
        <thead>
          <tr>
            {columns.map((col) => (
              <th key={col.key}>{col.header}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.map((row) => (
            <tr
              key={getRowKey(row)}
              className={getRowClassName ? getRowClassName(row) : ""}
            >
              {columns.map((col) => (
                <td key={col.key}>
                  {col.render ? col.render(row[col.key], row) : row[col.key]}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>

      {pagination && pagination.totalPages > 1 && (
        <div className="pagination">
          <button
            onClick={() => pagination.onPageChange(1)}
            disabled={pagination.page === 1}
            className="pagination-btn"
          >
            First
          </button>
          <button
            onClick={() => pagination.onPageChange(pagination.page - 1)}
            disabled={pagination.page === 1}
            className="pagination-btn"
          >
            Previous
          </button>
          <span className="pagination-info">
            Page {pagination.page} of {pagination.totalPages}
          </span>
          <button
            onClick={() => pagination.onPageChange(pagination.page + 1)}
            disabled={pagination.page === pagination.totalPages}
            className="pagination-btn"
          >
            Next
          </button>
          <button
            onClick={() => pagination.onPageChange(pagination.totalPages)}
            disabled={pagination.page === pagination.totalPages}
            className="pagination-btn"
          >
            Last
          </button>
        </div>
      )}
    </div>
  );
}

export default Table;
