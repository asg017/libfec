CREATE TABLE libfec_filings(
    /**
     * 
     * 
     */

    --- Unique numeric identifier for this filing, assigned by the FEC, ex 1884420
    filing_id TEXT PRIMARY KEY NOT NULL,
    
    --- Version of the FEC filing format, ex '8.4'
    fec_version TEXT NOT NULL,
    
    --- Name of the software that produced this filing, ex 'NetFile'
    software_name TEXT NOT NULL,

    --- Version of the software that produced this filing, ex '2022451'
    software_version TEXT NOT NULL,

    --- If this filing is an amendment, the report_id of the original filing, otherwise null. ex 1884419
    report_id INTEGER,

    --- Sequential number of amendments
    report_number TEXT,

    --- Any header comments provided by the filer
    comment TEXT,

    --- Form type of the cover record, ex 'F3'
    cover_record_form TEXT NOT NULL,
    cover_record_form_amendment_indicator TEXT,

    filer_id TEXT NOT NULL,
    filer_name TEXT NOT NULL,
    report_code TEXT,
    coverage_from_date TEXT,
    coverage_through_date TEXT
  )