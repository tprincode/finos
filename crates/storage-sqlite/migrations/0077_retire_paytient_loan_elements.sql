-- Owner DB may still have a Loan element named Paytient created outside the seed id.

DELETE FROM planned_occurrence
WHERE element_id IN (
    SELECT element_id FROM cash_element
    WHERE lower(trim(account)) = 'loan' AND lower(trim(note)) = 'paytient'
);

DELETE FROM cash_element
WHERE lower(trim(account)) = 'loan' AND lower(trim(note)) = 'paytient';
