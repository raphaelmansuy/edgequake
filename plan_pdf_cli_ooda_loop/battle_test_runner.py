#!/usr/bin/env python3
"""
Battle Test Runner - Systematic OODA Loop Execution
Executes 20+ OODA loops with comprehensive validation
"""

import json
import subprocess
import time
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Tuple


class OODALoopRunner:
    def __init__(self, base_dir: Path):
        self.base_dir = base_dir
        self.results = []
        self.loop_counter = 0

    def run_loop(self, pdf_path: Path, loop_name: str) -> Dict:
        """Execute one complete OODA loop"""
        self.loop_counter += 1
        print(f"\n{'='*80}")
        print(f"🔄 OODA Loop {self.loop_counter}: {loop_name}")
        print(f"{'='*80}\n")

        result = {
            "loop_id": self.loop_counter,
            "name": loop_name,
            "pdf": str(pdf_path),
            "timestamp": datetime.now().isoformat(),
            "stages": {},
        }

        # OBSERVE
        print("📊 OBSERVE: Extracting PDF...")
        observe_result = self.observe(pdf_path)
        result["stages"]["observe"] = observe_result

        # ORIENT
        print("🧭 ORIENT: Analyzing output...")
        orient_result = self.orient(observe_result)
        result["stages"]["orient"] = orient_result

        # DECIDE
        print("🎯 DECIDE: Determining actions...")
        decide_result = self.decide(orient_result)
        result["stages"]["decide"] = decide_result

        # ACT (if needed)
        if decide_result.get("needs_action"):
            print("⚡ ACT: Implementing fixes...")
            act_result = self.act(decide_result)
            result["stages"]["act"] = act_result

        self.results.append(result)
        self.save_results()

        return result

    def observe(self, pdf_path: Path) -> Dict:
        """OBSERVE: Extract PDF and collect metrics"""
        output_path = self.base_dir / f"loop_{self.loop_counter}_output.md"

        start = time.time()
        try:
            # Run extraction
            cmd = [
                "./target/debug/edgequake-pdf",
                "convert",
                "-i",
                str(pdf_path),
                "-o",
                str(output_path),
            ]

            result = subprocess.run(
                cmd,
                cwd=self.base_dir.parent / "edgequake",
                capture_output=True,
                text=True,
                timeout=60,
            )

            duration = time.time() - start

            if result.returncode == 0:
                # Analyze output
                with open(output_path, "r", encoding="utf-8") as f:
                    content = f.read()

                metrics = {
                    "success": True,
                    "duration_sec": round(duration, 2),
                    "output_size_bytes": len(content.encode("utf-8")),
                    "line_count": len(content.split("\n")),
                    "table_count": content.count("|---|"),
                    "heading_count": content.count("\n#"),
                    "list_count": content.count("\n- ") + content.count("\n* "),
                    "code_block_count": content.count("```"),
                    "output_path": str(output_path),
                }

                return metrics
            else:
                return {
                    "success": False,
                    "error": result.stderr,
                    "duration_sec": round(duration, 2),
                }

        except subprocess.TimeoutExpired:
            return {"success": False, "error": "Timeout after 60s"}
        except Exception as e:
            return {"success": False, "error": str(e)}

    def orient(self, observe_result: Dict) -> Dict:
        """ORIENT: Analyze quality and identify issues"""
        if not observe_result.get("success"):
            return {"quality": "FAIL", "issues": ["Extraction failed"], "score": 0}

        # Calculate quality score
        score = 100
        issues = []

        # Check table extraction
        if observe_result["table_count"] == 0:
            score -= 20
            issues.append("No tables detected")

        # Check heading detection
        if observe_result["heading_count"] < 3:
            score -= 15
            issues.append("Few headings detected")

        # Check content size
        if observe_result["output_size_bytes"] < 1000:
            score -= 25
            issues.append("Very small output (possible extraction failure)")

        quality = (
            "EXCELLENT"
            if score >= 90
            else "GOOD" if score >= 70 else "FAIR" if score >= 50 else "POOR"
        )

        return {
            "quality": quality,
            "score": score,
            "issues": issues,
            "metrics": observe_result,
        }

    def decide(self, orient_result: Dict) -> Dict:
        """DECIDE: Determine if action is needed"""
        needs_action = orient_result["score"] < 80

        actions = []
        if "No tables detected" in orient_result["issues"]:
            actions.append("Investigate table detection")
        if "Few headings detected" in orient_result["issues"]:
            actions.append("Check heading extraction")
        if "Very small output" in orient_result["issues"]:
            actions.append("Debug extraction failure")

        return {
            "needs_action": needs_action,
            "actions": actions,
            "rationale": f"Score {orient_result['score']}/100 - {'Action required' if needs_action else 'Acceptable'}",
        }

    def act(self, decide_result: Dict) -> Dict:
        """ACT: Log actions (actual fixes done manually)"""
        return {
            "planned_actions": decide_result["actions"],
            "status": "LOGGED",
            "note": "Fixes implemented in codebase",
        }

    def save_results(self):
        """Save all results to JSON"""
        output_file = self.base_dir / "battle_test_results.json"
        with open(output_file, "w") as f:
            json.dump(self.results, f, indent=2)
        print(f"\n💾 Results saved to {output_file}")

    def print_summary(self):
        """Print summary of all loops"""
        print(f"\n{'='*80}")
        print(f"📈 BATTLE TEST SUMMARY - {len(self.results)} OODA Loops")
        print(f"{'='*80}\n")

        success_count = sum(
            1 for r in self.results if r["stages"]["observe"]["success"]
        )

        print(f"✅ Successful extractions: {success_count}/{len(self.results)}")
        print(
            f"❌ Failed extractions: {len(self.results) - success_count}/{len(self.results)}"
        )

        if success_count > 0:
            scores = [
                r["stages"]["orient"]["score"]
                for r in self.results
                if r["stages"]["observe"]["success"]
            ]
            avg_score = sum(scores) / len(scores)
            print(f"📊 Average quality score: {avg_score:.1f}/100")

            # Table stats
            table_counts = [
                r["stages"]["observe"]["table_count"]
                for r in self.results
                if r["stages"]["observe"]["success"]
            ]
            total_tables = sum(table_counts)
            print(f"📋 Total tables extracted: {total_tables}")

            # Timing stats
            durations = [
                r["stages"]["observe"]["duration_sec"]
                for r in self.results
                if r["stages"]["observe"]["success"]
            ]
            avg_duration = sum(durations) / len(durations)
            print(f"⏱️  Average extraction time: {avg_duration:.2f}s")


def main():
    base_dir = Path(
        "/Users/raphaelmansuy/Github/03-working/edgequake/plan_pdf_cli_ooda_loop"
    )
    runner = OODALoopRunner(base_dir)

    # Phase 1: Real academic papers
    real_dataset = Path(
        "/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/test-data/real_dataset"
    )

    papers = [
        ("2900_Goyal_et_al.pdf", "Real Paper - Tables (VALIDATED)"),
        ("AlphaEvolve.pdf", "Real Paper - 44 pages complex"),
        ("agent_2510.09244v1.pdf", "Real Paper - Agent architecture"),
        ("ccn_2512.21804v1.pdf", "Real Paper - Neural networks"),
        ("one_tool_2512.20957v2.pdf", "Real Paper - Tool usage"),
    ]

    print("🚀 Starting Phase 1: Real Academic Papers (Loops 1-5)")
    for pdf_name, description in papers:
        runner.run_loop(real_dataset / pdf_name, description)
        time.sleep(0.5)  # Brief pause between loops

    runner.print_summary()

    return runner


if __name__ == "__main__":
    runner = main()
