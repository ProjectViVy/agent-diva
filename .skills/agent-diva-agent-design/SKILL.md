---
name: agent-diva-agent-design
description: Expert-level design, review, and orchestration of Agent Diva agents and subagents. Use this when you want to: (1) Design new Agents with advanced cognitive architectures (including Persona, ReAct/CoT strategies, RAG integration), (2) Plan complex multi-agent topologies (such as Supervisor, Swarm, Graph-based routing), (3) Decompose complex business logic into high-concurrency, state-controllable skill flows and message flows within the Rust core, (4) Design fault tolerance, self-reflection, and evaluation mechanisms.
---

# Agent Diva Agent Design (Expert Level)

## Overview

This Skill is specifically tailored for expert-level Agent design, review, and evolution within the **Agent Diva** architecture. It goes beyond basic attribute configuration, focusing heavily on **cognitive architecture design**, **multi-agent collaboration models**, **extreme context optimization**, and **production-grade stability** to ensure the delivered Agent system can handle complex, multi-step, non-deterministic tasks.

## Applicable Scenarios

- **Building complex cognitive entities from scratch**: For example, designing a deep-analysis Agent that requires a complete closed-loop of "Planning-Retrieval-Reasoning-Execution-Reflection".
- **Designing Multi-Agent Orchestration**: When a single Agent hits a bottleneck and requires the introduction of a Supervisor node or a Swarm network.
- **Refactoring and State Management Optimization**: Decomposing black-box long conversations into deterministic flows based on Finite State Machines (FSM) or Directed Acyclic Graphs (DAG).
- **Production-grade fault-tolerant design**: Introducing LLM-as-a-judge evaluation for critical paths and self-correction strategies for Tool call failures.

## Core Workflow (Workflow-Based)

### Step 0: Confirm Architecture Boundaries and Context
- **Deep reading of conventions**: Read `AGENTS.md`, `CLAUDE.md`to understand global constraints and the macro Rust architecture.
- **Outline I/O and Concurrency Models**: Confirm the interaction modes (streaming, async wait, bidirectional long connection) of the target channels (Telegram/Discord/CLI/Tauri UI).
- **Align Model Characteristics**: Evaluate the strengths of the selected Provider based on the **provider-model-id-safety** rules (e.g., Claude's long-context reasoning, function calling accuracy of specific models) to allocate tasks rationally.

### Step 1: Cognitive Architecture and Role Modeling
It's no longer just about "what to do/what not to do," but defining how it thinks:
- **Persona & Boundaries**: Clearly define the system persona, professional domain, and refusal boundaries (Guardrails).
- **Thinking Paradigm (Prompting Paradigm)**: Determine whether the Agent needs to be forced to adopt a specific reasoning framework (e.g., ReAct, Plan-and-Solve, Chain-of-Thought).

- **I/O Contracts**: Strictly define upstream trigger conditions and structured output constraints (e.g., enforced JSON Schema) to ensure safe downstream deserialization.

### Step 2: Topology & Orchestration
Combined with the Agent Diva architecture, design the message flow topology between components:
- **Topology Selection**:
  - **Network Type (Swarm)**: Agents are peer-to-peer, tossing messages to each other through intent recognition.
  - **Hierarchical Type (Supervisor)**: A master control Agent is responsible for decomposing tasks, dispatching them to Subagents (e.g., Executor, Retriever, Reviewer), and aggregating the results.
  - **Pipeline (Pipeline/Graph)**: Deterministic steps flowing strictly according to a graph (e.g., Extract Requirements -> Retrieve Data -> Anomaly Detection -> Generate Report).

- **Component Mapping**:
  - **agent-diva-core**: Design custom event types or State objects used to pass context between Subagents rather than just raw text.
  - **agent-diva-tools**: Define granular Function Calling interfaces to ensure parameter types strictly correspond to Rust Structs.
  - **agent-diva-providers**: Route different models for different Subagents (e.g., advanced models for the main controller, smaller models for simple classifiers to improve response speed).

### Step 3: Prompt Architecture and Dynamic Skill Assembly
1. **Modular Prompt Assembly**:
   - Base System Prompt (Persona and Red Lines).
   - Dynamic Context Injector (RAG snippets dynamically pulled from `MEMORY.md` or vector databases).
   - Current State Indicator (Tells the Agent which stage of the workflow it is currently in).
2. **Skill Loading**:
   - Based on task complexity, design dynamic loading/unloading strategies for `.workspace/SKILLS/*.md` to avoid Context Window pollution and Attention Dilution.
3. **Self-Reflection and Error Correction (Self-Correction)**:
   - Embed error handling logic into the Prompt: "If the tool you called returns an error or empty data, you must analyze the reason and try again with different parameters, up to 3 times."

### Step 4: Memory Stratification and State Persistence
- **Short Window (Working Memory)**: Design sliding window strategies or Token truncation mechanisms to retain the last N rounds of conversation and tool execution results.
- **Semantic Search/RAG (Episodic Memory)**: When involving a large amount of history or documents, design how to vectorize information or extract key Entities to store in the middle tier.
- **Long-Term Memory Distillation (Long-term Knowledge)**: Design "memory compression" triggers—automatically call a Subagent to extract conclusions and write them to `HISTORY.md` or `MEMORY.md` when the session ends or a specific Token threshold is reached.

### Step 5: Validation, Observability, and Iteration
- **Deterministic Testing Paths**: Design unit/integration tests for Agent logic in Rust (Mock Provider responses, test whether the toolchain is triggered correctly).
- **Failure Escape Routes (Fallback Strategy)**: Design degradation handling schemes for when the API times out, the model outputs gibberish, or it falls into an infinite loop.
- **Observable Logs**: It is recommended during the design phase to clarify which intermediate thinking processes (Thoughts) need to be recorded at the DEBUG level, and which need to be displayed to the end-user (e.g., Loading status or typewriter effects on the UI).

## Quick Call Examples
- "I want to design a complex review process, breaking a large Agent into four sub-agents: 'Master Control - Retrieval - Review - Summarization'. Please help me design the data flow and Rust trait interfaces based on the Supervisor pattern."
- "This Agent frequently crashes after a tool call fails. Help me redesign its Prompt and skill set to include self-reflection and retry mechanisms."
- "Please help me plan a scheme combining short and long-term memory so the Agent can periodically summarize conversations and persist them to MEMORY.md, while controlling Token overhead."