# Consistency Model (Read Consistency Scope)

This project aims to provide strong consistency guarantees using the Raft
consensus algorithm. However, being a learning project, read consistency
mechanisms like Hybrid Logical Clocks (HLCs) and closed timestamps are not
considered. Read consistency from replicas/non-leaders will rely on the
capabilities provided directly by the underlying Raft library.


