-- Fails while cancelled approvals exist: they cannot be represented without the new status.
ALTER TABLE order_approvals DROP CONSTRAINT order_approvals_status_check;
ALTER TABLE order_approvals
    ADD CONSTRAINT order_approvals_status_check CHECK (status IN ('APPROVED', 'REJECTED'));
