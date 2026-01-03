

# **Strategic Competitive Intelligence Dossier: Power BI Sentinel (Fabric Sentinel)**

## **1\. Executive Intelligence Summary**

### **1.1 Strategic Overview**

Power BI Sentinel, developed by UK-based consultancy Purple Frog Systems, has firmly established itself as a premier third-party governance and disaster recovery solution within the Microsoft Power BI ecosystem. Recently rebranding to "Fabric Sentinel" to align with Microsoft’s strategic unification of data analytics under the "Fabric" banner, the platform addresses critical gaps in Microsoft's native service offering—specifically regarding automated backups, historical change tracking, and extended audit log retention.

For a new entrant developing a multi-platform Excel/PBIX semantic analysis and difference engine, Sentinel represents a formidable incumbent in the "Governance" and "Administration" sectors, but it exhibits significant vulnerabilities in the "Development," "Engineering," and "Deep Semantic Logic" sectors. Sentinel’s architecture is built fundamentally as a **monitoring and recovery tool**, designed to safeguard assets after they have been deployed. It is not an **authoring or engineering tool** designed to facilitate the development process itself. This distinction is the primary strategic wedge for a new market entrant.

The analysis indicates that while Sentinel excels at broad, tenant-wide observability and disaster recovery for IT Directors and Administrators, it lacks the granular, code-level sophistication required by Analytics Engineers and Power BI Developers who manage complex CI/CD pipelines. Furthermore, Sentinel’s handling of Excel is peripheral—treating it largely as a data source rather than a calculation engine—leaving a massive market opportunity for a solution that can unify semantic logic analysis across the hybrid Excel-Power BI landscape.

### **1.2 Key Findings**

* **Market Position:** Sentinel acts as the "Safety Net" for the Power BI Service, targeting IT Governance teams rather than report developers. Its adoption is driven by fear of data loss and compliance mandates (ISO 27001/SOC 2).  
* **Technological limitations:** The platform relies on "snapshot-based" change tracking. It compares static backups rather than integrating with live Git repositories for branch management. Its "diff" capabilities are visual and text-based, lacking deep Abstract Syntax Tree (AST) logic analysis.  
* **The Excel Gap:** Sentinel provides lineage *from* Excel but offers zero visibility *into* Excel logic. It cannot diff cell formulas, VBA macros, or Power Pivot models within Excel, a critical oversight given the ubiquity of Excel in financial reporting.  
* **Fabric Evolution:** The transition to "Fabric Sentinel" expands its scope to include Data Factory pipelines and Notebooks, signaling a move toward "Capacity Governance" and away from purely report management.

---

## **2\. Corporate Profile and Market Positioning**

### **2.1 Origins and Development History**

Power BI Sentinel is a product of **Purple Frog Systems Ltd**, a data analytics consultancy based in Telford, Shropshire, UK.1 Founded in 2006 by Alex Whittles, a prominent Microsoft Data Platform MVP, the company initially focused on Business Intelligence consultancy before identifying a recurring market need for better governance tools in the rapidly expanding Power BI ecosystem.1

The tool’s genesis lies in the "wild west" era of Power BI adoption, where organizations faced "Power BI Hell"—a proliferation of unmanaged, undocumented reports similar to the "Excel Hell" of previous decades.3 Purple Frog Systems leveraged their consultancy experience to build a SaaS solution that automated the manual tasks their consultants were frequently hired to perform: documenting datasets, backing up files, and tracing data lineage.

This consultancy background is evident in the product’s design philosophy. It is pragmatic, prioritizing "quick wins" for administrators (like generating a PDF documentation file for an auditor) over complex features for developers (like merging code branches). The leadership team, including Directors Alex and Hollie Whittles, maintains close ties to the Microsoft community, frequently speaking at events like SQLBits, which helps sustain the product’s visibility and credibility within the Microsoft partner channel.5

### **2.2 Target Audience and Personas**

The marketing and feature set of Sentinel are distinctively oriented toward high-level governance rather than low-level code authoring. The platform targets three distinct personas:

#### **The IT Director / CISO**

* **Pain Point:** "What happens if a rogue employee deletes our financial reporting workspace?" or "How do we prove to the auditor who accessed this PII data?"  
* **Sentinel Solution:** Automated, immutable backups stored in the client’s own Azure tenant and long-term retention of audit logs beyond Microsoft’s 90-day default.1

#### **The BI Administrator**

* **Pain Point:** "I have 5,000 reports in my tenant. I don't know which ones are used, which are broken, or who owns them."  
* **Sentinel Solution:** Tenant-wide inventory, unused report identification to reclaim license costs, and automated lineage scanning to map dependencies.8

#### **The "Accidental" BI Developer**

* **Pain Point:** "I changed a measure and the report broke, but I don't know what the code was yesterday."  
* **Sentinel Solution:** A simple, visual "Change Tracking" interface that shows a before-and-after view without requiring knowledge of Git or version control systems.9

### **2.3 Commercial Strategy**

Sentinel positions itself as a low-friction, high-value add-on to the Microsoft stack. It explicitly markets against the complexity of building custom governance solutions. While a skilled engineer could replicate some Sentinel features using PowerShell and the Power BI REST APIs, Sentinel argues that the Total Cost of Ownership (TCO) of maintaining those scripts against a constantly changing API surface is higher than their license fee.10

Furthermore, they position themselves as a cost-effective alternative to **Microsoft Purview**. While Purview is an enterprise-wide data governance catalog often criticized for its complexity and cost, Sentinel offers "deep" Power BI governance at a fraction of the price and setup time, claiming to be up and running in "minutes not months".11

---

## **3\. Technical Architecture and Security Model**

A robust evaluation of Sentinel requires a detailed dissection of its architecture. It operates as a SaaS platform but employs a unique hybrid data residency model designed to appease enterprise security teams.

### **3.1 The Hybrid SaaS / BYOS Architecture**

Sentinel operates on a **"Bring Your Own Storage" (BYOS)** model. This is a critical architectural decision that separates the *processing* of governance data from the *storage* of sensitive intellectual property.

#### **The Processing Layer (Sentinel Cloud)**

The core application logic runs in Purple Frog’s Azure tenant. This layer handles:

* **API Polling:** Scheduled jobs that query the customer’s Power BI tenant via REST APIs.9  
* **Metadata Extraction:** Parsing of report layouts, DAX expressions, and M code.13  
* **Comparison Logic:** The compute power required to run diffs between historical versions.

#### **The Storage Layer (Customer Cloud)**

Sentinel does not store the actual backups (.pbix or .abf files) on its own infrastructure. Instead, customers must provision their own **Azure Blob Storage** account and grant Sentinel access via a connection string or SAS token.13

* **Security Implication:** This ensures **Data Sovereignty**. The physical files containing the company's data never rest on Sentinel’s servers. They are streamed directly from Microsoft’s cloud to the customer’s storage container. If a customer cancels their subscription, they retain 100% of their backups.14

#### **The Reporting Layer (Azure SQL)**

Customers also provision an **Azure SQL Database**. Sentinel writes extracted metadata (lineage, audit logs, usage stats) into this database.

* **Strategic Advantage:** This "Open Data" approach allows customers to connect their own Power BI reports to the SQL DB to build custom governance dashboards. It avoids the "walled garden" problem where governance data is locked inside a proprietary vendor tool.8

### **3.2 Service Principal and Permissions Model**

To function effectively, Sentinel requires extensive privileges within the customer’s tenant. It recommends the use of a **Service Principal** (an Azure AD App Registration) rather than a user account to avoid MFA prompts and password expiry issues.16

The permission scopes required are extensive and often a point of friction during security reviews:

* Tenant.Read.All: To inventory all workspaces and artifacts.  
* Tenant.ReadWrite.All: Required if the Service Principal needs to add itself to workspaces to perform exports.18  
* Dataset.ReadWrite.All: Necessary for triggering backups and interacting with the XMLA endpoint.  
* **High-Privilege Risk:** Sentinel attempts to mitigate the risk of requiring such high privileges by recommending that the Service Principal be managed via **Privileged Identity Management (PIM)** or restricted via security groups, but functionally, the tool requires "God-mode" visibility to deliver its full value proposition of tenant-wide disaster recovery.19

### **3.3 Data Residency and Geo-Redundancy**

To comply with GDPR and other data localization laws, Sentinel processes data in specific Azure regions.

* **Supported Regions:** US Central, EU (Europe), and Australia.8  
* **Mechanism:** When a user logs in, they select their region (e.g., portal.powerbisentinel.com vs eu.portal...). The processing logic remains within that geopolitical boundary. Combined with the BYOS model (where the customer chooses the region of their own Azure Storage), this allows Sentinel to serve highly regulated industries like finance and healthcare that strictly forbid data from leaving a specific jurisdiction.21

### **3.4 Security Certifications and Trust**

Sentinel maintains **ISO 27001** certification and aligns with **SOC 2** standards, which are prerequisites for selling into the enterprise tier.23 The platform undergoes regular third-party penetration testing. The company leverages its "Metadata Only" processing model as a primary security argument: strictly speaking, Sentinel processes the *structure* of the data (schema, report layout) but does not query or store the *rows* of data within the reports, except transiently during the backup stream to the customer's storage.23

---

## **4\. Feature Deep Dive: Disaster Recovery (DR) and Backups**

Disaster Recovery is the foundational utility of the Sentinel platform. In the native Power BI Service, if a user deletes a report or overwrites it with a corrupted version, there is no "Undo" button. Microsoft’s native recovery options are limited to soft-delete recovery of entire workspaces (for a short window) or reliance on local file versions, which may not exist in a self-service environment.

### **4.1 Automated PBIX Export Mechanics**

Sentinel utilizes the Power BI REST API (Export Report) to download copies of reports and datasets.

* **Scheduling:** Administrators can define schedules (Daily, Weekly) for critical workspaces. Sentinel iterates through the workspace, identifies changed artifacts, and exports the .pbix file.9  
* **Versioning Chain:** Files are saved in the Azure Blob Storage with timestamps (e.g., Sales\_Report\_2023-10-25.pbix). This creates an immutable history, allowing administrators to access any previous version.13

### **4.2 Handling Large Models and API Limitations**

A critical competitive nuance is how Sentinel handles artifacts that *cannot* be exported as PBIX files. Microsoft blocks PBIX downloads for:

1. **Large Models:** Datasets exceeding the 1GB download limit or using Large Dataset Storage Format.  
2. **Incremental Refresh:** Datasets with incremental refresh policies active.  
3. **XMLA Modifications:** Datasets modified by external tools like Tabular Editor.

Sentinel addresses this via **ABF (Analysis Services Backup File)** support.24

* **The XMLA Workaround:** For Premium/Fabric workspaces, Sentinel connects to the XMLA endpoint (the interface for the underlying Analysis Services engine) and triggers a database backup.  
* **The Result:** This generates an .abf file instead of a .pbix file.  
* **The Trade-off:** While this ensures the *data model* is backed up, an .abf file does not contain the *report visuals*. If a large report is deleted, restoring the .abf recovers the data, but the visual layout might be lost unless Sentinel also managed to export a "Thin Report" (a PBIX with no data, just visuals). This fragmentation is a significant complexity in the recovery process that Sentinel mitigates but cannot entirely solve due to platform limitations.26

### **4.3 Recovery Workflows**

The recovery process in Sentinel is manual and asynchronous.

* **Download to Restore:** To recover a file, the administrator must log into the Sentinel portal, find the historical version, and download it to their local machine.  
* **Republish:** They must then manually republish this file to the Power BI Service via Power BI Desktop.  
* **No "In-Place" Restore:** Sentinel cannot "right-click and restore" a report directly in the service. The Power BI APIs do not support overwriting a report from an external source without changing its internal GUIDs or requiring a complex "Import" operation that might break existing dashboard tiles. Thus, Sentinel is a *backup* tool, but the *restore* is a manual human operation.9

### **4.4 Reliability and Failure Modes**

Analysis of support documentation reveals several common failure modes that a competitor could exploit by offering better resilience or diagnostics:

* **Throttling:** Large tenants often hit API throttling limits, causing backups to skip or fail. Sentinel’s logs often show ExportData\_DisabledByTenant or generic timeout errors.25  
* **Permission Decay:** If the Service Principal is removed from a workspace (e.g., by a workspace admin cleaning up users), backups fail silently until the next audit check.  
* **Configuration Drift:** If a dataset is switched to "Large Storage Format," PBIX backups immediately fail. Sentinel requires the user to manually reconfigure that workspace to use ABF backups, a friction point that an intelligent engine could automate.25

---

## **5\. Feature Deep Dive: Change Tracking and Semantic Analysis**

For the user’s specific interest in a "diff engine," this section provides the most critical competitive data. Sentinel’s change tracking is a "monitoring" implementation, distinct from a "development" implementation.

### **5.1 The "Snapshot" Comparison Methodology**

Sentinel does not integrate with Git or track code commits in real-time. Instead, it relies on a **Snapshot Methodology**.

* **Process:** When a backup is taken (e.g., nightly), Sentinel parses the metadata of the new file and compares it against the metadata of the previous version stored in its database.  
* **Latency:** This means change tracking is typically **Daily**, not real-time. If a user makes five changes between 9 AM and 5 PM, Sentinel only sees the net difference at the next scheduled scan. It misses the intermediate states, which is a significant disadvantage compared to a tool integrated into a CI/CD pipeline.9

### **5.2 Visualization of Changes (UI Analysis)**

The Change Tracking UI presents a list-based view of differences.9

* **Visuals:** It detects changes in the JSON layout configuration. It can flag that a "Bar Chart" became a "Line Chart" or that a visual was moved. It attempts to filter out "noise" (e.g., minor pixel movements), but users still report high signal-to-noise ratios in visual diffing.9  
* **Data Model (DAX):** It extracts the DAX expression for measures and calculated columns. The diff view shows a text comparison (often side-by-side or Red/Green text blocks) of the formula.  
* **Power Query (M):** It tracks changes to the underlying M code of queries.

### **5.3 Semantic Intelligence Gaps**

Sentinel’s analysis is primarily **syntactic** rather than **semantic**.

* **Text vs. Logic:** If a user adds a comment to a DAX measure or changes the formatting (indentation), Sentinel likely flags this as a change because the text string is different. A true "Semantic Engine" (like ALM Toolkit) normalizes the code to ignore whitespace and formatting, focusing only on functional logic changes. Sentinel appears to lack this depth of normalization.29  
* **Dependency Tracing:** Sentinel can tell you that "Measure A changed" and "Visual B changed." It relies on the user to infer that Visual B changed *because* Measure A was altered. It does not explicitly map the **causal link** in the diff view (e.g., "Visual B is broken because Measure A’s data type changed from Integer to String").

### **5.4 The "Read-Only" Limitation**

Crucially, Sentinel is a **Read-Only** system.

* **No Merge:** It allows administrators to *see* what changed, but they cannot selectively *merge* changes. They cannot say, "Keep the new visual layout but revert the DAX measure." The only action available is to download the old file and overwrite the new one entirely.9  
* **Comparison to Developer Tools:** This places it distinctly behind tools like **ALM Toolkit** (which allows granular merging of model metadata) and **Tabular Editor** (which allows code authoring). Sentinel is for the *observer*, not the *builder*.

---

## **6\. Data Governance, Lineage, and Impact Analysis**

### **6.1 The Lineage Architecture**

Sentinel markets "Enhanced Data Lineage" as a key differentiator. While Power BI has a native lineage view, Sentinel extends this by utilizing the Service Principal to scan *all* workspaces, including "Personal" workspaces that are typically invisible to standard admins.3

#### **The Graph vs. The Tree**

* **Graph View:** Sentinel provides a visual node-link diagram showing the flow from Gateway \-\> Server \-\> Database \-\> Report \-\> Dashboard \-\> App.15  
* **Data Source Explorer (Tree View):** Recognizing that graphs become unreadable in large tenants, Sentinel offers a hierarchical tree view. An admin can find a specific SQL Server instance and expand it to see every report connecting to it. This "Server-centric" view is highly valued by DBAs planning migrations or decommissioning servers.8

### **6.2 The "Column-Level Lineage" Claim**

Sentinel claims to support **Column Level Lineage**, a notorious challenge in Power BI due to the complexity of the M (Power Query) language.31

* **Implementation Reality:** Research suggests this is achieved via **Regular Expression (Regex) parsing** of the source queries.32  
* **The Fragility of Regex:** By parsing the text of the SQL query embedded in the M code, Sentinel can identify that SELECT CustomerID FROM Customers uses the CustomerID column. However, this approach is brittle.  
  * It struggles with SELECT \*.  
  * It fails on complex dynamic SQL constructed at runtime.  
  * It cannot easily trace a column if it is renamed three times during Power Query transformations (e.g., Col1 \-\> Rev \-\> GrossRevenue).  
* **Competitive Opportunity:** A new engine that utilizes a true M-language parser (Abstract Syntax Tree) or hooks into the query execution plan would offer significantly higher accuracy and reliability than Sentinel’s text-parsing approach.

### **6.3 Automated Documentation**

Sentinel generates "Data Dictionary" style documentation.3

* **Artifacts:** Automated PDF or HTML reports listing every table, column, measure, and description in a dataset.  
* **Use Case:** This is primarily a compliance artifact. It allows teams to "tick the box" for documentation requirements without manually maintaining Word documents.

---

## **7\. Auditing and Compliance Capabilities**

### **7.1 Long-Term Log Retention**

A primary driver for Sentinel adoption is the limitation of Microsoft's native audit logs, which (depending on the license) are often retained for only 90 days.

* **Ingestion Pipeline:** Sentinel continuously polls the **Office 365 Unified Audit Log** and **Power BI Activity Log**.  
* **Storage:** It writes these events into the customer’s Azure SQL Database. Since the customer owns the DB, retention is indefinite.  
* **Enrichment:** Sentinel enriches the raw logs. A raw log might show DatasetID: GUID-123. Sentinel joins this with its inventory snapshot to add DatasetName: Sales\_2024, making the log human-readable for auditors.33

### **7.2 Permission Auditing and "Over-Privileged" Users**

Sentinel scans the user access lists for every artifact.

* **Recursive Group Resolution:** It integrates with the Microsoft Graph API to resolve Active Directory Security Groups. If "Finance Team" has access to a workspace, Sentinel lists the individual members of that group. This granularity is essential for audits where the question is "Does John Doe have access?" not "Does the Finance Team have access?".8

### **7.3 Compliance Reporting**

Sentinel provides pre-built Power BI report templates that sit on top of the customer’s SQL database. These templates include:

* **"Who has seen what?"** (GDPR Access Request).  
* **"Unused Reports"** (Clean-up candidates).  
* **"External User Access"** (Security risk assessment).

---

## **8\. Fabric Integration: The Evolution to "Fabric Sentinel"**

As Microsoft unifies its data stack under **Microsoft Fabric**, Sentinel has pivoted to remain relevant.

### **8.1 Fabric Artifact Inventory**

"Fabric Sentinel" now supports the inventory and tracking of non-Power BI artifacts.19

* **Scope:** Data Pipelines, Notebooks, Lakehouses, Warehouses, and Spark Definitions.  
* **Capacity Monitoring:** A key new feature is monitoring **Capacity Unit (CU)** consumption. With Fabric’s pay-per-capacity model, organizations are desperate to identify which specific item is draining their budget. Sentinel is positioning itself to provide this granular cost attribution, competing with Microsoft's own Fabric Metrics App by offering longer history and better alerting.19

### **8.2 Intra-Day Refresh Monitoring**

For enterprise clients (Tier E-1000+), Sentinel has introduced **Intra-Day Refresh Processing**.

* **The Need:** Standard Sentinel scans are daily. For a Fabric environment where data moves continuously, daily is too slow.  
* **The Feature:** It checks refresh statuses multiple times per day, providing near real-time alerting on pipeline failures, a critical requirement for Data Engineering teams vs. the slower cadence of BI reporting.19

---

## **9\. Commercial Analysis: Pricing and Tiers**

Sentinel employs a tiered pricing model based on the "Number of Reports" in the tenant. This creates a scalable model that captures both mid-market and enterprise segments.

### **9.1 Licensing Structure**

| Tier | Target Report Count | Approx. Annual Cost | Target Customer | Key Features |
| :---- | :---- | :---- | :---- | :---- |
| **Core (C-250)** | Up to 250 | \~£3,600 ($4,700) | Small Business / Dept | Backups, Change Tracking |
| **Core (C-500)** | Up to 500 | \~£6,000 ($7,800) | Mid-Market | \+ Lineage, Documentation |
| **Enterprise (E-1000)** | Up to 1,000 | \~£9,500 ($12,350) | Enterprise | \+ Fabric Support, Intra-day |
| **Enterprise (E-5000)** | Up to 5,000 | \~£26,000 ($34,200) | Large Enterprise | Scale support, Priority SLAs |
| **Dedicated Host** | Unlimited / Custom | Custom Quote | Regulated (Gov/Fin) | Private Azure Instance |

Data derived from.35 Prices are approximate and subject to exchange rates/updates.

### **9.2 Market Reception and ROI**

* **Positive Sentiment:** Reviews cite the low cost relative to the "salary cost" of building internal tools. Ideally, an internal build would require a Data Engineer to maintain scripts; Sentinel costs less than 10% of that engineer’s salary. Users appreciate the "peace of mind" and the ease of setup.11  
* **Negative Sentiment:** Some users find the UI functional but dated. Support can be variable depending on the complexity of the issue (e.g., specific API failure codes). Enterprise users with massive tenants (\>50k reports) sometimes face performance lags in the scan times.8

---

## **10\. Competitive Landscape**

### **10.1 Sentinel vs. ALM Toolkit**

* **ALM Toolkit** is a developer tool for **Merging** and **Diffing** semantic models.  
* **Contrast:** ALM Toolkit is "Pre-Deployment." It is used *before* you publish to see what changed and merge branches. Sentinel is "Post-Deployment." It is used *after* you publish to see what is currently there.  
* **Feature Gap:** Sentinel cannot merge. ALM Toolkit cannot backup files or track visuals. They are complementary, not substitutes, though they both offer "Diff" views.31

### **10.2 Sentinel vs. Tabular Editor**

* **Tabular Editor** is an **Authoring** tool.  
* **Contrast:** You use Tabular Editor to write code and script changes. You use Sentinel to document those changes for the auditor. Sentinel’s backup of XMLA-modified datasets relies on the compatibility maintained by tools like Tabular Editor.31

### **10.3 Sentinel vs. Microsoft Purview**

* **Microsoft Purview** is the enterprise governance catalog.  
* **Contrast:** Purview is broad (SQL, Oracle, SAP, Power BI) but often shallow in Power BI specifics (e.g., lacking DAX diffs or automated PBIX backups). Sentinel is deep in Power BI but narrow. Sentinel markets itself as "Better than Purview for Power BI, and Cheaper".38

### **10.4 Sentinel vs. Custom PowerShell**

* **Custom Scripts** are the "Build" alternative.  
* **Contrast:** Scripts require maintenance. When Microsoft changes the API (which happens frequently), the script breaks. Sentinel absorbs this maintenance cost. However, scripts offer infinite flexibility (e.g., "Trigger a backup, then email me, then run a test"), which Sentinel’s fixed feature set cannot match.39

---

## **11\. Strategic Recommendations for the New Engine**

The analysis of Power BI Sentinel reveals a distinct "identity" for the product: it is a **safety net for administrators**. This leaves a massive strategic opening for a tool designed as a **force multiplier for engineers**.

### **11.1 The "Excel Logic" Opportunity**

Sentinel treats Excel as a second-class citizen—primarily as a data source file.

* **The Gap:** In Finance and Actuarial science, logic is often split between complex Excel models (Power Pivot/VBA) and Power BI. Sentinel cannot diff the logic inside an Excel cell.  
* **The Move:** The new engine should treat Excel as a first-class semantic citizen. It should parse Excel formulas and VBA, creating a unified dependency graph that says: "This Power BI Report Metric depends on Cell C5 in this Excel Model, which changed logic today." This capability would effectively lock out Sentinel from the "End-User Computing" (EUC) governance market.

### **11.2 The "Semantic Logic" vs. "Text Diff" Opportunity**

Sentinel’s diffs are textual.

* **The Gap:** If a developer refactors code to be more performant but functionally identical, Sentinel flags it as a change.  
* **The Move:** The new engine should implement **Abstract Syntax Tree (AST)** comparison. It should detect "Logic Equivalence" versus "Logic Divergence." It should provide performance implications of the diff (e.g., "You removed a filter context; this query will now scan 10x more rows"). This moves the value proposition from "Tracking" to "Intelligence."

### **11.3 The "CI/CD Integration" Opportunity**

Sentinel sits *outside* the development pipeline. It watches the production environment.

* **The Gap:** Mature data teams want to catch errors *before* they deploy. Sentinel cannot block a deployment.  
* **The Move:** The new engine should be API/CLI-first, designed to run inside GitHub Actions or Azure DevOps pipelines. It should act as a **Quality Gate**: "Block deployment because the semantic diff shows a breaking change in a core KPI." This targets the "Pro Code" market that Sentinel ignores.30

### **11.4 The "Conflict Resolution" Opportunity**

Sentinel is Read-Only.

* **The Gap:** Teams working on the same PBIX file face "Merge Hell." Sentinel offers no help here.  
* **The Move:** The new engine should offer a "Three-Way Merge" interface for PBIX/TMDL files, allowing developers to resolve conflicts visually. This solves the single biggest pain point in collaborative Power BI development.

---

Conclusion  
Power BI Sentinel is a formidable incumbent in the "Backup and Audit" space. It has successfully captured the market of IT Directors seeking insurance against data loss. However, it is fundamentally a passive monitoring tool. A new multi-platform engine that focuses on active development intelligence—deep semantic diffing, cross-platform Excel/PBI logic mapping, and CI/CD integration—would address the sophisticated requirements of the modern Analytics Engineer, a segment that Sentinel’s current architecture is ill-equipped to serve.

#### **Works cited**

1. About Us \- Purple Frog Systems, accessed November 28, 2025, [https://www.purplefrogsystems.com/about-us/](https://www.purplefrogsystems.com/about-us/)  
2. Purple Frog Systems Ltd \- Company Profile \- Endole, accessed November 28, 2025, [https://open.endole.co.uk/insight/company/05672331-purple-frog-systems-ltd](https://open.endole.co.uk/insight/company/05672331-purple-frog-systems-ltd)  
3. Power BI Sentinel \- Purple Frog Systems, accessed November 28, 2025, [https://www.purplefrogsystems.com/2019/03/power-bi-sentinel/](https://www.purplefrogsystems.com/2019/03/power-bi-sentinel/)  
4. Power BI Consulting | Reporting | Training \- Purple Frog Systems, accessed November 28, 2025, [https://www.purplefrogsystems.com/power-bi/](https://www.purplefrogsystems.com/power-bi/)  
5. Bristol Microsoft Fabric User Group, accessed November 28, 2025, [https://community.fabric.microsoft.com/t5/Bristol-Microsoft-Fabric-User/gh-p/pbi\_bristol\_usergroup](https://community.fabric.microsoft.com/t5/Bristol-Microsoft-Fabric-User/gh-p/pbi_bristol_usergroup)  
6. Hoppy Times in the Big Apple: Power BI Sentinel's visit to New York City, accessed November 28, 2025, [https://www.purplefrogsystems.com/2024/05/data-universe-2024/](https://www.purplefrogsystems.com/2024/05/data-universe-2024/)  
7. Power BI Disaster Recovery \- Automated Backups and Change Tracking, accessed November 28, 2025, [https://www.powerbisentinel.com/powerbi-disaster-recovery/](https://www.powerbisentinel.com/powerbi-disaster-recovery/)  
8. Providing tailored insights and information relevant to your needs as a Technical User. \- Power BI Sentinel, accessed November 28, 2025, [https://www.powerbisentinel.com/for-technical-users/](https://www.powerbisentinel.com/for-technical-users/)  
9. Change Tracking \- Power BI Sentinel, accessed November 28, 2025, [https://www.powerbisentinel.com/change-tracking/](https://www.powerbisentinel.com/change-tracking/)  
10. Power BI Gateway Monitoring \- why is it so bad? : r/PowerBI \- Reddit, accessed November 28, 2025, [https://www.reddit.com/r/PowerBI/comments/srb143/power\_bi\_gateway\_monitoring\_why\_is\_it\_so\_bad/](https://www.reddit.com/r/PowerBI/comments/srb143/power_bi_gateway_monitoring_why_is_it_so_bad/)  
11. Power BI Sentinel, accessed November 28, 2025, [https://www.powerbisentinel.com/](https://www.powerbisentinel.com/)  
12. Optimised Power BI Data Governance Services \- BoomData, accessed November 28, 2025, [https://www.boomdata.com.au/power-bi-governance/](https://www.boomdata.com.au/power-bi-governance/)  
13. Power BI Sentinel: Backup, Documentation, Change Tracking And Lineage Tracking For Power BI \- Chris Webb's BI Blog, accessed November 28, 2025, [https://blog.crossjoin.co.uk/2019/03/19/power-bi-sentinel-backup-documentation-change-tracking-and-lineage-tracking-for-power-bi/](https://blog.crossjoin.co.uk/2019/03/19/power-bi-sentinel-backup-documentation-change-tracking-and-lineage-tracking-for-power-bi/)  
14. Setting Up Power BI Sentinel, accessed November 28, 2025, [https://www.powerbisentinel.com/setting-up-power-bi-sentinel/](https://www.powerbisentinel.com/setting-up-power-bi-sentinel/)  
15. Power BI Lineage Explorer \- Showing you the way\!, accessed November 28, 2025, [https://www.powerbisentinel.com/power-bi-lineage/](https://www.powerbisentinel.com/power-bi-lineage/)  
16. Creating a Service Principal and Connecting to Power BI, accessed November 28, 2025, [https://www.powerbisentinel.com/creating-a-service-principal-and-connecting-to-power-bi/](https://www.powerbisentinel.com/creating-a-service-principal-and-connecting-to-power-bi/)  
17. Creating a service principal for Power BI \- YouTube, accessed November 28, 2025, [https://www.youtube.com/watch?v=XmWTUPAW55w](https://www.youtube.com/watch?v=XmWTUPAW55w)  
18. What Permissions to the Power BI Service does Power BI Sentinel need?, accessed November 28, 2025, [https://www.powerbisentinel.com/helpdesk/what-permissions-to-our-power-bi-service-does-power-bi-sentinel-need/](https://www.powerbisentinel.com/helpdesk/what-permissions-to-our-power-bi-service-does-power-bi-sentinel-need/)  
19. What's New? \- Power BI Sentinel, accessed November 28, 2025, [https://www.powerbisentinel.com/whats-new/](https://www.powerbisentinel.com/whats-new/)  
20. Geographical availability and data residency in Microsoft Sentinel, accessed November 28, 2025, [https://learn.microsoft.com/en-us/azure/sentinel/geographical-availability-data-residency](https://learn.microsoft.com/en-us/azure/sentinel/geographical-availability-data-residency)  
21. Azure Maps Power BI visual Data Residency \- Microsoft Learn, accessed November 28, 2025, [https://learn.microsoft.com/en-us/azure/azure-maps/power-bi-visual-data-residency](https://learn.microsoft.com/en-us/azure/azure-maps/power-bi-visual-data-residency)  
22. Setup Under Strict Data Residency Requirements : r/PowerBI \- Reddit, accessed November 28, 2025, [https://www.reddit.com/r/PowerBI/comments/fmbq9a/setup\_under\_strict\_data\_residency\_requirements/](https://www.reddit.com/r/PowerBI/comments/fmbq9a/setup_under_strict_data_residency_requirements/)  
23. How secure is the application? \- Power BI Sentinel, accessed November 28, 2025, [https://www.powerbisentinel.com/helpdesk/how-secure-is-the-application/](https://www.powerbisentinel.com/helpdesk/how-secure-is-the-application/)  
24. Power BI Power BI Backups \- Power BI Sentinel \- Safe and Secure to your storage, accessed November 28, 2025, [https://www.powerbisentinel.com/power-bi-backups/](https://www.powerbisentinel.com/power-bi-backups/)  
25. Diagnose your backup failure codes \- Power BI Sentinel, accessed November 28, 2025, [https://www.powerbisentinel.com/helpdesk/diagnose-your-backup-failure-codes/](https://www.powerbisentinel.com/helpdesk/diagnose-your-backup-failure-codes/)  
26. ABF Backups Overview \- Power BI Sentinel, accessed November 28, 2025, [https://www.powerbisentinel.com/helpdesk/summary-of-abf-backups/](https://www.powerbisentinel.com/helpdesk/summary-of-abf-backups/)  
27. ️ Troubleshooting Power BI Sentinel Backup Failures, accessed November 28, 2025, [https://www.powerbisentinel.com/helpdesk/troubleshooting-power-bi-sentinel-backup-failures/](https://www.powerbisentinel.com/helpdesk/troubleshooting-power-bi-sentinel-backup-failures/)  
28. Need Power BI Assistance? Learn How Power BI Sentinel Help You \- YouTube, accessed November 28, 2025, [https://www.youtube.com/watch?v=9rDAAHkbWac](https://www.youtube.com/watch?v=9rDAAHkbWac)  
29. Comparing and finding differences in pbix files and GIT integration : r/PowerBI \- Reddit, accessed November 28, 2025, [https://www.reddit.com/r/PowerBI/comments/hlhgls/comparing\_and\_finding\_differences\_in\_pbix\_files/](https://www.reddit.com/r/PowerBI/comments/hlhgls/comparing_and_finding_differences_in_pbix_files/)  
30. Power BI DevOps, accessed November 28, 2025, [https://www.powerbisentinel.com/power-bi-devops/](https://www.powerbisentinel.com/power-bi-devops/)  
31. T-SQL Tuesday \#135 \- My Tools for the Trade \- Benni De Jagere, accessed November 28, 2025, [https://bennidejagere.com/2021/02/t-sql-tuesday-135-my-tools-for-the-trade/](https://bennidejagere.com/2021/02/t-sql-tuesday-135-my-tools-for-the-trade/)  
32. Alex Whittles, Author at Purple Frog Systems, accessed November 28, 2025, [https://www.purplefrogsystems.com/author/alex/](https://www.purplefrogsystems.com/author/alex/)  
33. Power BI Auditing, Usage Analytics & Logging, accessed November 28, 2025, [https://www.powerbisentinel.com/usage-logging/](https://www.powerbisentinel.com/usage-logging/)  
34. Fabric Sentinel – Is it the new Power BI Sentinel, accessed November 28, 2025, [https://www.powerbisentinel.com/fabric-sentinel/](https://www.powerbisentinel.com/fabric-sentinel/)  
35. Pricing 2025 \- Power BI Sentinel, accessed November 28, 2025, [https://www.powerbisentinel.com/pricing/](https://www.powerbisentinel.com/pricing/)  
36. Microsoft Sentinel Pros and Cons | User Likes & Dislikes \- G2, accessed November 28, 2025, [https://www.g2.com/products/microsoft-sentinel/reviews?qs=pros-and-cons](https://www.g2.com/products/microsoft-sentinel/reviews?qs=pros-and-cons)  
37. Is there any we can have a VCS system for Power BI reports or any work around? \- Reddit, accessed November 28, 2025, [https://www.reddit.com/r/PowerBI/comments/ph3kxy/is\_there\_any\_we\_can\_have\_a\_vcs\_system\_for\_power/](https://www.reddit.com/r/PowerBI/comments/ph3kxy/is_there_any_we_can_have_a_vcs_system_for_power/)  
38. 6 Key Pillars For Successful Self Service Analytics \- BoomData, accessed November 28, 2025, [https://www.boomdata.com.au/blog/6-key-pillars-for-successful-self-service-analytics/](https://www.boomdata.com.au/blog/6-key-pillars-for-successful-self-service-analytics/)  
39. Calling Power BI Admins\! Admin Monitoring and Addons : r/PowerBI \- Reddit, accessed November 28, 2025, [https://www.reddit.com/r/PowerBI/comments/1midxod/calling\_power\_bi\_admins\_admin\_monitoring\_and/](https://www.reddit.com/r/PowerBI/comments/1midxod/calling_power_bi_admins_admin_monitoring_and/)  
40. Microsoft Fabric, Power BI, Analysis Services, DAX, M, MDX, Power Query, Power Pivot and Excel \- Chris Webb's BI Blog, accessed November 28, 2025, [https://blog.crossjoin.co.uk/page/40/?tl=1&](https://blog.crossjoin.co.uk/page/40/?tl=1&)