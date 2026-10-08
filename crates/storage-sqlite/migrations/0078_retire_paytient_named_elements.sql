-- Paytient is paid off. Remove any recurring element still named Paytient
-- (owner had one on the Health book, not only Loan).

DELETE FROM planned_occurrence
WHERE element_id IN (
    SELECT element_id FROM cash_element
    WHERE lower(trim(note)) = 'paytient'
);

DELETE FROM cash_element
WHERE lower(trim(note)) = 'paytient';
