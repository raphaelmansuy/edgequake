#!/usr/bin/env python3
"""
Phase 2: Synthetic PDF Suite Testing (Loops 6-20)
Tests all synthetic PDFs from test-data directory
"""

import subprocess
import json
import time
from pathlib import Path
from datetime import datetime
from typing import Dict, List
import sys

def run_synthetic_suite():
    """Test all synthetic PDFs from test-data"""
    
    base_dir = Path("/Users/raphaelmansuy/Github/03-working/edgequake")
    test_data = base_dir / "edgequake/crates/edgequake-pdf/test-data"
    output_dir = base_dir / "plan_pdf_cli_ooda_loop"
    
    # Find all synthetic PDFs (numbered)
    pdfs = sorted([p for p in test_data.glob("0*.pdf") if p.stem.split('_')[0].isdigit()])
    
    print(f"🚀 Phase 2: Synthetic PDF Suite - Found {len(pdfs)} test PDFs")
    print(f"{'='*80}\n")
    
    results = []
    loop_id = 6  # Starting from loop 6 (after Phase 1)
    
    for pdf in pdfs:
        print(f"\n🔄 OODA Loop {loop_id}: {pdf.name}")
        print(f"-" * 80)
        
        result = {
            "loop_id": loop_id,
            "pdf": pdf.name,
            "timestamp": datetime.now().isoformat()
        }
        
        # OBSERVE: Extract
        output_path = output_dir / f"loop_{loop_id}_output.md"
        start = time.time()
        
        try:
            cmd = [
                "./target/debug/edgequake-pdf",
                "convert",
                "-i", str(pdf),
                "-o", str(output_path)
            ]
            
            proc = subprocess.run(
                cmd,
                cwd=base_dir / "edgequake",
                capture_output=True,
                text=True,
                timeout=30
            )
            
            duration = time.time() - start
            
            if proc.returncode == 0:
                with open(output_path, 'r') as f:
                    content = f.read()
                
                # Check for gold standard
                gold_file = pdf.with_suffix('.gold.md')
                has_gold = gold_file.exists()
                
                result["success"] = True
                result["duration_sec"] = round(duration, 2)
                result["size_bytes"] = len(content.encode('utf-8'))
                result["tables"] = content.count('|---|')
                result["headings"] = content.count('\n#')
                result["has_gold"] = has_gold
                
                print(f"✅ Success: {duration:.2f}s, {len(content)} chars, {result['tables']} tables")
                
            else:
                result["success"] = False
                result["error"] = proc.stderr[:200]
                print(f"❌ Failed: {proc.stderr[:100]}")
                
        except subprocess.TimeoutExpired:
            result["success"] = False
            result["error"] = "Timeout"
            print(f"❌ Timeout after 30s")
        except Exception as e:
            result["success"] = False
            result["error"] = str(e)
            print(f"❌ Error: {str(e)[:100]}")
        
        results.append(result)
        loop_id += 1
        
        if loop_id % 5 == 0:
            # Save checkpoint every 5 loops
            checkpoint_file = output_dir / f"phase2_checkpoint_{loop_id}.json"
            with open(checkpoint_file, 'w') as f:
                json.dump(results, f, indent=2)
            print(f"\n💾 Checkpoint saved: {loop_id-6} loops complete")
    
    # Final summary
    print(f"\n{'='*80}")
    print(f"📈 PHASE 2 COMPLETE - {len(results)} Synthetic PDFs")
    print(f"{'='*80}\n")
    
    success_count = sum(1 for r in results if r.get("success"))
    print(f"✅ Successful: {success_count}/{len(results)} ({success_count/len(results)*100:.1f}%)")
    print(f"❌ Failed: {len(results)-success_count}/{len(results)}")
    
    if success_count > 0:
        total_tables = sum(r.get("tables", 0) for r in results if r.get("success"))
        total_headings = sum(r.get("headings", 0) for r in results if r.get("success"))
        avg_duration = sum(r.get("duration_sec", 0) for r in results if r.get("success")) / success_count
        
        print(f"📋 Total tables extracted: {total_tables}")
        print(f"📑 Total headings extracted: {total_headings}")
        print(f"⏱️  Average time: {avg_duration:.2f}s")
        
        # Save final results
        final_file = output_dir / "phase2_synthetic_results.json"
        with open(final_file, 'w') as f:
            json.dump({
                "summary": {
                    "total": len(results),
                    "success": success_count,
                    "failed": len(results) - success_count,
                    "success_rate": f"{success_count/len(results)*100:.1f}%",
                    "total_tables": total_tables,
                    "total_headings": total_headings,
                    "avg_duration_sec": round(avg_duration, 2)
                },
                "results": results
            }, f, indent=2)
        
        print(f"\n💾 Full results saved to {final_file}")
    
    return results

if __name__ == "__main__":
    results = run_synthetic_suite()
    sys.exit(0 if all(r.get("success") for r in results) else 1)
