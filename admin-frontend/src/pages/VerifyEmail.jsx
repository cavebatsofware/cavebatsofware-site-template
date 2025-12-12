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

import React, { useEffect, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { fetchApi } from '../utils/api';

function VerifyEmail() {
  const [searchParams] = useSearchParams();
  const [status, setStatus] = useState('verifying');
  const [error, setError] = useState('');
  const [email, setEmail] = useState('');

  useEffect(() => {
    verifyEmail();
  }, []);

  async function verifyEmail() {
    const token = searchParams.get('token');

    if (!token) {
      setStatus('error');
      setError('No verification token provided');
      return;
    }

    try {
      const response = await fetchApi(`/api/admin/verify-email?token=${token}`, {
        method: 'GET',
      });

      if (!response.ok) {
        const data = await response.json();
        throw new Error(data.error || 'Verification failed');
      }

      const data = await response.json();
      setEmail(data.email);
      setStatus('success');
    } catch (err) {
      setStatus('error');
      setError(err.message);
    }
  }

  if (status === 'verifying') {
    return (
      <div className="container">
        <div className="card">
          <h1>Verifying Email...</h1>
          <p>Please wait while we verify your email address.</p>
        </div>
      </div>
    );
  }

  if (status === 'error') {
    return (
      <div className="container">
        <div className="card">
          <div className="error">
            <h1>Verification Failed</h1>
            <p>{error}</p>
          </div>
          <div className="link">
            <Link to="/register">Register Again</Link> or{' '}
            <Link to="/login">Login</Link>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="container">
      <div className="card">
        <div className="success">
          <h1>Email Verified!</h1>
          <p>
            Your email <strong>{email}</strong> has been successfully verified.
          </p>
          <p>You can now log in to your admin account.</p>
        </div>
        <div className="link">
          <Link to="/login">Go to Login</Link>
        </div>
      </div>
    </div>
  );
}

export default VerifyEmail;
