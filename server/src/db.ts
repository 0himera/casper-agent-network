import mysql from 'mysql2/promise';

const uri = process.env.DB_URI || 'mysql://root:password@127.0.0.1:3306/deagentnet';

export const pool = mysql.createPool({
  uri,
  waitForConnections: true,
  connectionLimit: 10,
  queueLimit: 0,
});
