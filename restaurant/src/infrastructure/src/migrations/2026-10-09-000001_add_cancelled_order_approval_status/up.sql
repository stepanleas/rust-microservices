-- Saga compensation: an approved order can be cancelled later (its stock is given back).
ALTER TABLE order_approvals DROP CONSTRAINT order_approvals_status_check;
ALTER TABLE order_approvals
    ADD CONSTRAINT order_approvals_status_check CHECK (status IN ('APPROVED', 'REJECTED', 'CANCELLED'));
